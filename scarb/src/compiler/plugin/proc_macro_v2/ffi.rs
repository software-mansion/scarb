use crate::compiler::plugin::proc_macro_common::{Expansion, ExpansionKind};
use crate::core::PackageId;
use anyhow::{ensure, Context, Result};
use cairo_lang_macro_stable_v2::{
    StableExpansion, StableExpansionsList, StablePostProcessContext, StableProcMacroResult,
    StableResultWrapper, StableTextSpan, StableTokenStream,
};
use cairo_lang_macro_v2::ExpansionKind as ExpansionKindV2;
use cairo_lang_macro_v2::{
    FullPathMarker, PostProcessContext, ProcMacroResult, TextSpan, TokenStream,
};
use camino::Utf8Path;
use itertools::Itertools;
use libloading::{Library, Symbol};
use std::ffi::{c_char, CStr, CString};
use std::slice;

use crate::compiler::plugin::proc_macro_v2::ProcMacroAuxData;
#[cfg(not(windows))]
use libloading::os::unix::Symbol as RawSymbol;
#[cfg(windows)]
use libloading::os::windows::Symbol as RawSymbol;
use smol_str::SmolStr;

type ListExpansions = extern "C" fn() -> StableExpansionsList;
type FreeExpansionsList = extern "C" fn(StableExpansionsList);
type ExpandCode = extern "C" fn(
    *const c_char,
    StableTextSpan,
    StableTokenStream,
    StableTokenStream,
) -> StableResultWrapper;
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
        Ok(VTableV0 {
            list_expansions: get_symbol!(library, b"list_expansions_v2\0", ListExpansions),
            free_expansions_list: get_symbol!(
                library,
                b"free_expansions_list_v2\0",
                FreeExpansionsList
            ),
            expand: get_symbol!(library, b"expand_v2\0", ExpandCode),
            free_result: get_symbol!(library, b"free_result_v2\0", FreeResult),
            post_process_callback: get_symbol!(
                library,
                b"post_process_callback_v2\0",
                PostProcessCallback
            ),
            doc: get_symbol!(library, b"doc_v2\0", DocExpansion),
            free_doc: get_symbol!(library, b"free_doc_v2\0", FreeExpansionDoc),
        })
    }
}

pub struct Plugin {
    #[allow(dead_code)]
    library: Library,
    vtable: VTableV0,
}

impl Plugin {
    pub(crate) unsafe fn try_new(library_path: &Utf8Path) -> Result<Plugin> {
        let library = Library::new(library_path)?;
        let vtable = VTableV0::try_new(&library)?;

        Ok(Plugin { library, vtable })
    }

    pub(crate) unsafe fn load_expansions(&self, package_id: PackageId) -> Result<Vec<Expansion>> {
        // Make a call to the FFI interface to list declared expansions.
        let stable_expansions = (self.vtable.list_expansions)();
        let (ptr, n) = stable_expansions.raw_parts();
        let expansions = slice::from_raw_parts(ptr, n);
        let mut expansions: Vec<Expansion> = expansions.iter().map(|e| e.into()).collect();
        // Free the memory allocated by the `stable_expansions`.
        (self.vtable.free_expansions_list)(stable_expansions);
        // Validate expansions.
        expansions.sort_unstable_by_key(|e| e.name.clone());
        ensure!(
            expansions.windows(2).all(|w| w[0].name != w[1].name),
            "duplicate expansions defined for procedural macro {package_id}: {duplicates}",
            duplicates = expansions
                .windows(2)
                .filter(|w| w[0].name == w[1].name)
                .map(|w| w[0].name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        Ok(expansions)
    }
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
        unsafe { PostProcessContext::free_owned_stable(context) };
    }

    pub fn doc(&self, item_name: SmolStr) -> Option<String> {
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
    /// Apply expansion to token stream.
    ///
    /// This function implements the actual calls to functions from the dynamic library.
    ///
    /// All values passing the FFI-barrier must implement a stable ABI.
    ///
    /// Please be aware that the memory management of values passing the FFI-barrier is tricky.
    /// The memory must be freed on the same side of the barrier, where the allocation was made.
    pub(crate) fn generate_code(
        &self,
        item_name: SmolStr,
        call_site: TextSpan,
        attr: TokenStream,
        token_stream: TokenStream,
    ) -> ProcMacroResult {
        // This must be manually freed with call to `free_owned_stable`.
        let stable_token_stream = token_stream.as_stable();
        let stable_attr = attr.as_stable();
        // Allocate proc macro name.
        let item_name = CString::new(item_name.to_string()).unwrap().into_raw();
        // Call FFI interface for code expansion.
        // Note that `stable_result` has been allocated by the dynamic library.
        let call_site: StableTextSpan = call_site.into_stable();
        let stable_result =
            (self.vtable.expand)(item_name, call_site, stable_attr, stable_token_stream);
        // Free proc macro name.
        let _ = unsafe { CString::from_raw(item_name) };
        // Free the memory allocated by the `stable_token_stream`.
        // This will call `CString::from_raw` under the hood, to take ownership.
        unsafe {
            TokenStream::free_owned_stable(stable_result.input);
            TokenStream::free_owned_stable(stable_result.input_attr);
        };
        // Create Rust representation of the result.
        // Note, that the memory still needs to be freed on the allocator side!
        let result = unsafe { ProcMacroResult::from_stable(&stable_result.output) };
        // Call FFI interface to free the `stable_result` that has been allocated by previous call.
        (self.vtable.free_result)(stable_result.output);
        // Return obtained result.
        result
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
        if name.starts_with(crate::compiler::plugin::proc_macro_common::EXEC_ATTR_PREFIX) {
            let name = name
                .strip_prefix(crate::compiler::plugin::proc_macro_common::EXEC_ATTR_PREFIX)
                .unwrap();
            return Self {
                name: SmolStr::new(name),
                kind: ExpansionKind::Executable,
            };
        }
        let expansion_kind = unsafe { ExpansionKindV2::from_stable(&stable_expansion.kind) }.into();
        Self {
            name: SmolStr::new(name),
            kind: expansion_kind,
        }
    }
}
