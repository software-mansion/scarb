use crate::compiler::plugin::proc_macro::v1::ProcMacroAuxData;
use crate::core::PackageId;
use anyhow::{Context, Result, ensure};
use cairo_lang_defs::patcher::PatchBuilder;
use cairo_lang_macro_stable_v1::{
    StableExpansion, StableExpansionsList, StablePostProcessContext, StableProcMacroResult,
    StableResultWrapper, StableTokenStream,
};
use cairo_lang_macro_v1::{
    ExpansionKind as ExpansionKindV1, FullPathMarker, PostProcessContext, ProcMacroResult,
    TokenStream,
};
use cairo_lang_syntax::node::TypedSyntaxNode;
use cairo_lang_syntax::node::db::SyntaxGroup;
use convert_case::{Case, Casing};
use itertools::Itertools;
use libloading::{Library, Symbol};
use std::ffi::{CStr, CString, c_char};
use std::slice;

use crate::compiler::plugin::proc_macro::expansion::{Expansion, ExpansionKind};
#[cfg(not(windows))]
use libloading::os::unix::Symbol as RawSymbol;
#[cfg(windows)]
use libloading::os::windows::Symbol as RawSymbol;
use smol_str::{SmolStr, ToSmolStr};

pub trait FromSyntaxNode {
    fn from_syntax_node(db: &dyn SyntaxGroup, node: &impl TypedSyntaxNode) -> Self;
}

impl FromSyntaxNode for TokenStream {
    fn from_syntax_node(db: &dyn SyntaxGroup, node: &impl TypedSyntaxNode) -> Self {
        let mut builder = PatchBuilder::new(db, node);
        builder.add_node(node.as_syntax_node());
        Self::new(builder.build().0)
    }
}

/// This constant is used to identify executable attributes.
///
/// An attribute is considered executable if it starts with this prefix.
/// No other metadata is stored for executable attributes.
/// This means, that this constant is part of the stable contract between Scarb and procedural macro.
/// Warning: Changing this would be breaking to existing macros!
const EXEC_ATTR_PREFIX: &str = "__exec_attr_";

type ListExpansions = extern "C" fn() -> StableExpansionsList;
type FreeExpansionsList = extern "C" fn(StableExpansionsList);
type ExpandCode =
    extern "C" fn(*const c_char, StableTokenStream, StableTokenStream) -> StableResultWrapper;
type FreeResult = extern "C" fn(StableProcMacroResult);
type PostProcessCallback = extern "C" fn(StablePostProcessContext) -> StablePostProcessContext;
type DocExpansion = extern "C" fn(*const c_char) -> *mut c_char;
type FreeExpansionDoc = extern "C" fn(*mut c_char);

struct VTableV0 {
    list_expansions: RawSymbol<ListExpansions>,
    free_expansions_list: RawSymbol<FreeExpansionsList>,
    expand: RawSymbol<ExpandCode>,
    free_result: RawSymbol<FreeResult>,
    post_process_callback: RawSymbol<PostProcessCallback>,
    doc: RawSymbol<DocExpansion>,
    free_doc: RawSymbol<FreeExpansionDoc>,
}

macro_rules! get_symbol {
    ($library:ident, $name:literal, $type:ty) => {{
        let symbol: Symbol<'_, $type> = $library.get($name).context(format!(
            "failed to load {} symbol for procedural macro",
            stringify!($name)
        ))?;
        symbol.into_raw()
    }};
}

impl VTableV0 {
    unsafe fn try_new(library: &Library) -> Result<VTableV0> {
        unsafe {
            Ok(VTableV0 {
                list_expansions: get_symbol!(library, b"list_expansions\0", ListExpansions),
                free_expansions_list: get_symbol!(
                    library,
                    b"free_expansions_list\0",
                    FreeExpansionsList
                ),
                expand: get_symbol!(library, b"expand\0", ExpandCode),
                free_result: get_symbol!(library, b"free_result\0", FreeResult),
                post_process_callback: get_symbol!(
                    library,
                    b"post_process_callback\0",
                    PostProcessCallback
                ),
                doc: get_symbol!(library, b"doc\0", DocExpansion),
                free_doc: get_symbol!(library, b"free_doc\0", FreeExpansionDoc),
            })
        }
    }
}

/// This struct is a wrapper around a shared library containing the procedural macro implementation.
/// It is responsible for loading the shared library and providing a safe interface for its access.
pub struct Plugin {
    #[allow(dead_code)]
    library: Library,
    vtable: VTableV0,
}

impl Plugin {
    /// Load the shared library under the given path and store pointers to its public symbols.
    ///
    /// # Safety
    /// This function is unsafe because it calls the FFI interface of procedural macro package.
    pub unsafe fn try_new(library: Library) -> Result<Plugin> {
        let vtable = unsafe { VTableV0::try_new(&library)? };
        Ok(Plugin { library, vtable })
    }

    /// Obtain metadata of available expansions from the procedural macro.
    ///
    /// # Safety
    /// This function is unsafe because it calls the FFI interface of procedural macro package.
    pub unsafe fn load_expansions(&self, package_id: PackageId) -> Result<Vec<Expansion>> {
        // Make a call to the FFI interface to list declared expansions.
        let stable_expansions = (self.vtable.list_expansions)();
        let (ptr, n) = stable_expansions.raw_parts();
        let expansions = unsafe { slice::from_raw_parts(ptr, n) };
        let mut expansions: Vec<Expansion> = expansions
            .iter()
            .map(|stable_expansion| stable_expansion.into())
            .collect();
        // Free the memory allocated by the `stable_expansions`.
        (self.vtable.free_expansions_list)(stable_expansions);
        // Validate expansions.
        expansions.sort_unstable_by_key(|e| e.cairo_name.clone());
        ensure!(
            expansions
                .windows(2)
                .all(|w| w[0].cairo_name != w[1].cairo_name),
            "duplicate expansions defined for procedural macro {package_id}: {duplicates}",
            duplicates = expansions
                .windows(2)
                .filter(|w| w[0].cairo_name == w[1].cairo_name)
                .map(|w| w[0].cairo_name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        Ok(expansions)
    }

    /// Apply expansion to token stream.
    ///
    /// This function implements the actual calls to functions from the dynamic library.
    ///
    /// All values passing the FFI-barrier must implement a stable ABI.
    ///
    /// Please be aware that the memory management of values passing the FFI-barrier is tricky.
    /// The memory must be freed on the same side of the barrier, where the allocation was made.
    ///
    /// # Safety
    /// This function is unsafe because it calls the FFI interface of procedural macro package.
    #[tracing::instrument(level = "trace", skip_all)]
    pub(crate) fn generate_code(
        &self,
        item_name: SmolStr,
        attr: TokenStream,
        token_stream: TokenStream,
    ) -> ProcMacroResult {
        // This must be manually freed with call to from_owned_stable.
        let stable_token_stream = token_stream.into_stable();
        let stable_attr = attr.into_stable();
        // Allocate proc macro name.
        let item_name = CString::new(item_name.to_string()).unwrap().into_raw();
        // Call FFI interface for code expansion.
        // Note that `stable_result` has been allocated by the dynamic library.
        let stable_result = (self.vtable.expand)(item_name, stable_attr, stable_token_stream);
        // Free proc macro name.
        let _ = unsafe { CString::from_raw(item_name) };
        // Free the memory allocated by the `stable_token_stream`.
        // This will call `CString::from_raw` under the hood, to take ownership.
        unsafe {
            TokenStream::from_owned_stable(stable_result.input);
            TokenStream::from_owned_stable(stable_result.input_attr);
        };
        // Create Rust representation of the result.
        // Note, that the memory still needs to be freed on the allocator side!
        let result = unsafe { ProcMacroResult::from_stable(&stable_result.output) };
        // Call FFI interface to free the `stable_result` that has been allocated by previous call.
        (self.vtable.free_result)(stable_result.output);
        // Return obtained result.
        result
    }

    /// Call post process callbacks defined in the procedural macro.
    ///
    /// # Safety
    /// This function is unsafe because it calls the FFI interface of procedural macro package.
    pub(crate) fn post_process_callback(
        &self,
        aux_data: Vec<ProcMacroAuxData>,
        full_path_markers: Vec<FullPathMarker>,
    ) {
        // Create stable representation of the context.
        let context = PostProcessContext {
            aux_data: aux_data.into_iter().map(Into::into).collect_vec(),
            full_path_markers,
        }
        .into_stable();
        // Actual call to FFI interface for aux data callback.
        let context = (self.vtable.post_process_callback)(context);
        // Free the allocated memory.
        let _ = unsafe { PostProcessContext::from_owned_stable(context) };
    }

    /// Obtain expansion doc string.
    ///
    /// # Safety
    /// This function is unsafe because it calls the FFI interface of procedural macro package.
    pub fn doc(&self, item_name: impl ToString) -> Option<String> {
        // Allocate proc macro name.
        let item_name = CString::new(item_name.to_string()).unwrap().into_raw();
        // Call FFI interface for expansion doc.
        // Note that `stable_result` has been allocated by the dynamic library.
        let stable_result = (self.vtable.doc)(item_name);
        let doc = if stable_result.is_null() {
            None
        } else {
            let cstr = unsafe { CStr::from_ptr(stable_result) };
            Some(cstr.to_string_lossy().to_string())
        };
        // Free proc macro name.
        let _ = unsafe { CString::from_raw(item_name) };
        // Call FFI interface to free the `stable_result` that has been allocated by previous call.
        (self.vtable.free_doc)(stable_result);
        doc
    }
}

impl From<&StableExpansion> for Expansion {
    fn from(stable_expansion: &StableExpansion) -> Self {
        // Note this does not take ownership of underlying memory.
        let name = if stable_expansion.name.is_null() {
            String::default()
        } else {
            let cstr = unsafe { CStr::from_ptr(stable_expansion.name) };
            cstr.to_string_lossy().to_string()
        };
        // Handle special case for executable attributes.
        if name.starts_with(EXEC_ATTR_PREFIX) {
            let name = name.strip_prefix(EXEC_ATTR_PREFIX).unwrap();
            let name = name.to_smolstr();
            return Self {
                cairo_name: name.clone(),
                expansion_name: name.clone(),
                kind: ExpansionKind::Executable,
            };
        }
        let expansion_kind = unsafe { ExpansionKindV1::from_stable(&stable_expansion.kind) }.into();
        let cairo_name = if matches!(expansion_kind, ExpansionKind::Derive) {
            let name = name.to_case(Case::UpperCamel);
            name.to_smolstr()
        } else {
            name.to_smolstr()
        };
        Self {
            cairo_name,
            expansion_name: name.to_smolstr(),
            kind: expansion_kind,
        }
    }
}
