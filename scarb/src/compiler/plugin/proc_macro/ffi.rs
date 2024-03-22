use crate::core::{Config, Package, PackageId};
use anyhow::{ensure, Context, Result};
use cairo_lang_defs::patcher::PatchBuilder;
use cairo_lang_macro::{AuxData, ExpansionKind, ProcMacroResult, TokenStream};
use cairo_lang_macro_stable::{
    StableAuxData, StableExpansion, StableExpansionsList, StableProcMacroResult,
    StableResultWrapper, StableTokenStream,
};
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::{ast, TypedSyntaxNode};
use camino::Utf8PathBuf;
use libloading::{Library, Symbol};
use std::ffi::{c_char, CStr, CString};
use std::fmt::Debug;
use std::slice;

use crate::compiler::plugin::proc_macro::compilation::SharedLibraryProvider;
use crate::compiler::plugin::proc_macro::ProcMacroAuxData;
use cairo_lang_macro_stable::ffi::StableSlice;
#[cfg(not(windows))]
use libloading::os::unix::Symbol as RawSymbol;
#[cfg(windows)]
use libloading::os::windows::Symbol as RawSymbol;
use smol_str::SmolStr;

pub trait FromItemAst {
    fn from_item_ast(db: &dyn SyntaxGroup, item_ast: ast::ModuleItem) -> Self;
}

impl FromItemAst for TokenStream {
    fn from_item_ast(db: &dyn SyntaxGroup, item_ast: ast::ModuleItem) -> Self {
        let mut builder = PatchBuilder::new(db);
        builder.add_node(item_ast.as_syntax_node());
        Self::new(builder.code)
    }
}

/// Representation of a single procedural macro.
///
/// This struct is a wrapper around a shared library containing the procedural macro implementation.
/// It is responsible for loading the shared library and providing a safe interface for code expansion.
pub struct ProcMacroInstance {
    package_id: PackageId,
    plugin: Plugin,
    expansions: Vec<Expansion>,
}

impl Debug for ProcMacroInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcMacroInstance")
            .field("package_id", &self.package_id)
            .finish()
    }
}

impl ProcMacroInstance {
    /// Load shared library
    pub fn try_new(package: Package, config: &Config) -> Result<Self> {
        let lib_path = package.shared_lib_path(config);
        let plugin = unsafe { Plugin::try_new(lib_path.to_path_buf())? };
        Ok(Self {
            expansions: unsafe { Self::load_expansions(&plugin, package.id)? },
            package_id: package.id,
            plugin,
        })
    }

    unsafe fn load_expansions(plugin: &Plugin, package_id: PackageId) -> Result<Vec<Expansion>> {
        // Make a call to the FFI interface to list declared expansions.
        let stable_expansions = (plugin.vtable.list_expansions)();
        let (ptr, n) = stable_expansions.raw_parts();
        let expansions = slice::from_raw_parts(ptr, n);
        let mut expansions: Vec<Expansion> = expansions
            .iter()
            .map(|e| Expansion::from_stable(e))
            .collect();
        // Free the memory allocated by the `stable_expansions`.
        (plugin.vtable.free_expansions_list)(stable_expansions);
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

    pub fn get_expansions(&self) -> &[Expansion] {
        &self.expansions
    }

    pub fn package_id(&self) -> PackageId {
        self.package_id
    }

    pub fn declared_attributes(&self) -> Vec<String> {
        self.get_expansions()
            .iter()
            .filter(|e| e.kind == ExpansionKind::Attr)
            .map(|e| e.name.clone())
            .map(Into::into)
            .collect()
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
        token_stream: TokenStream,
    ) -> ProcMacroResult {
        // This must be manually freed with call to from_owned_stable.
        let stable_token_stream = token_stream.into_stable();
        // Allocate proc macro name.
        let item_name = CString::new(item_name.to_string()).unwrap().into_raw();
        // Call FFI interface for code expansion.
        // Note that `stable_result` has been allocated by the dynamic library.
        let stable_result = (self.plugin.vtable.expand)(item_name, stable_token_stream);
        // Free proc macro name.
        let _ = unsafe { CString::from_raw(item_name) };
        // Free the memory allocated by the `stable_token_stream`.
        // This will call `CString::from_raw` under the hood, to take ownership.
        unsafe {
            TokenStream::from_owned_stable(stable_result.input);
        };
        // Create Rust representation of the result.
        // Note, that the memory still needs to be freed on the allocator side!
        let result = unsafe { ProcMacroResult::from_stable(&stable_result.output) };
        // Call FFI interface to free the `stable_result` that has been allocated by previous call.
        (self.plugin.vtable.free_result)(stable_result.output);
        // Return obtained result.
        result
    }

    pub(crate) fn aux_data_callback(&self, aux_data: Vec<ProcMacroAuxData>) {
        // Convert to stable aux data.
        let aux_data: Vec<AuxData> = aux_data.into_iter().map(Into::into).collect();
        let aux_data = aux_data
            .into_iter()
            .map(|a| a.into_stable())
            .collect::<Vec<_>>();
        // Create stable slice representation from vector.
        // Note this needs to be freed manually.
        let aux_data = StableSlice::new(aux_data);
        // Actual call to FFI interface for aux data callback.
        let aux_data = (self.plugin.vtable.aux_data_callback)(aux_data);
        // Free the memory allocated by vec.
        let _ = aux_data.into_owned();
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Expansion {
    pub name: SmolStr,
    pub kind: ExpansionKind,
}

impl Expansion {
    pub fn new(name: impl ToString, kind: ExpansionKind) -> Self {
        Self {
            name: SmolStr::new(name.to_string()),
            kind,
        }
    }

    unsafe fn from_stable(stable_expansion: &StableExpansion) -> Self {
        // Note this does not take ownership of underlying memory.
        let name = if stable_expansion.name.is_null() {
            String::default()
        } else {
            let cstr = CStr::from_ptr(stable_expansion.name);
            cstr.to_string_lossy().to_string()
        };
        Self {
            name: SmolStr::new(name),
            kind: ExpansionKind::from_stable(&stable_expansion.kind),
        }
    }
}

type ListExpansions = extern "C" fn() -> StableExpansionsList;
type FreeExpansionsList = extern "C" fn(StableExpansionsList);
type ExpandCode = extern "C" fn(*const c_char, StableTokenStream) -> StableResultWrapper;
type FreeResult = extern "C" fn(StableProcMacroResult);
type AuxDataCallback = extern "C" fn(StableSlice<StableAuxData>) -> StableSlice<StableAuxData>;

struct VTableV0 {
    list_expansions: RawSymbol<ListExpansions>,
    free_expansions_list: RawSymbol<FreeExpansionsList>,
    expand: RawSymbol<ExpandCode>,
    free_result: RawSymbol<FreeResult>,
    aux_data_callback: RawSymbol<AuxDataCallback>,
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
            list_expansions: get_symbol!(library, b"list_expansions\0", ListExpansions),
            free_expansions_list: get_symbol!(
                library,
                b"free_expansions_list\0",
                FreeExpansionsList
            ),
            expand: get_symbol!(library, b"expand\0", ExpandCode),
            free_result: get_symbol!(library, b"free_result\0", FreeResult),
            aux_data_callback: get_symbol!(library, b"aux_data_callback\0", AuxDataCallback),
        })
    }
}

struct Plugin {
    #[allow(dead_code)]
    library: Library,
    vtable: VTableV0,
}

impl Plugin {
    unsafe fn try_new(library_path: Utf8PathBuf) -> Result<Plugin> {
        let library = Library::new(library_path)?;
        let vtable = VTableV0::try_new(&library)?;

        Ok(Plugin { library, vtable })
    }
}
