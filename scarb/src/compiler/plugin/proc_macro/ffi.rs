use crate::core::Package;
use cairo_lang_defs::patcher::PatchBuilder;
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::{ast, TypedSyntaxNode};
use scarb_macro_interface::{ProcMacroResult, TokenStream};
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
pub struct ProcMacroInstance {}

impl ProcMacroInstance {
    pub fn try_new(_package: Package) -> anyhow::Result<Self> {
        // Load shared library
        // TODO(maciektr): Implement
        Ok(Self {})
    }

    pub(crate) fn generate_code(&self, _token_stream: TokenStream) -> ProcMacroResult {
        // Apply expansion to token stream.
        // TODO(maciektr): Implement
        ProcMacroResult::Leave
    }
}
