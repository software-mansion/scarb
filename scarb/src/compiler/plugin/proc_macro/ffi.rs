use crate::core::{Config, Package, PackageId};
use anyhow::{Context, Result};
use cairo_lang_defs::patcher::PatchBuilder;
use cairo_lang_macro::{ProcMacroResult, TokenStream};
use cairo_lang_macro_stable::{StableProcMacroResult, StableTokenStream};
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::{ast, TypedSyntaxNode};
use camino::Utf8PathBuf;
use libloading::{Library, Symbol};
use std::fmt::Debug;

use crate::compiler::plugin::proc_macro::compilation::SharedLibraryProvider;
#[cfg(not(windows))]
use libloading::os::unix::Symbol as RawSymbol;
#[cfg(windows)]
use libloading::os::windows::Symbol as RawSymbol;

pub const PROC_MACRO_BUILD_PROFILE: &str = "release";

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
}

impl Debug for ProcMacroInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcMacroInstance")
            .field("package_id", &self.package_id)
            .finish()
    }
}

impl ProcMacroInstance {
    pub fn package_id(&self) -> PackageId {
        self.package_id
    }

    /// Load shared library
    pub fn try_new(package: Package, config: &Config) -> Result<Self> {
        let lib_path = package.shared_lib_path(config);
        let plugin = unsafe { Plugin::try_new(lib_path.to_path_buf())? };
        Ok(Self {
            plugin,
            package_id: package.id,
        })
    }
    pub fn declared_attributes(&self) -> Vec<String> {
        vec![self.package_id.name.to_string()]
    }

    /// Apply expansion to token stream.
    pub(crate) fn generate_code(&self, token_stream: TokenStream) -> ProcMacroResult {
        let ffi_token_stream = token_stream.into_stable();
        let result = (self.plugin.vtable.expand)(ffi_token_stream);
        unsafe { ProcMacroResult::from_stable(result) }
    }
}

type ExpandCode = extern "C" fn(StableTokenStream) -> StableProcMacroResult;

struct VTableV0 {
    expand: RawSymbol<ExpandCode>,
}

impl VTableV0 {
    unsafe fn try_new(library: &Library) -> Result<VTableV0> {
        let expand: Symbol<'_, ExpandCode> = library
            .get(b"expand\0")
            .context("failed to load expand function for procedural macro")?;
        let expand = expand.into_raw();
        Ok(VTableV0 { expand })
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
