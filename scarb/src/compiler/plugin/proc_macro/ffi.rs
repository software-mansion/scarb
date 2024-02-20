use crate::core::{Package, PackageId};
use anyhow::Result;
use cairo_lang_defs::patcher::PatchBuilder;
use cairo_lang_macro::{ProcMacroResult, TokenStream};
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::{ast, TypedSyntaxNode};
use std::fmt::Debug;

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
#[derive(Debug, Clone)]
pub struct ProcMacroInstance {
    package_id: PackageId,
}

impl ProcMacroInstance {
    pub fn package_id(&self) -> PackageId {
        self.package_id
    }

    pub fn try_new(package: Package) -> Result<Self> {
        // Load shared library
        // TODO(maciektr): Implement
        Ok(Self {
            package_id: package.id,
        })
    }

    pub fn declared_attributes(&self) -> Vec<String> {
        vec![self.package_id.name.to_string()]
    }

    pub(crate) fn generate_code(&self, _token_stream: TokenStream) -> ProcMacroResult {
        // Apply expansion to token stream.
        // TODO(maciektr): Implement
        ProcMacroResult::Leave
    }
}
