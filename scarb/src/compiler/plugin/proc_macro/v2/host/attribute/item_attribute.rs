use crate::compiler::plugin::proc_macro::v2::host::attribute::child_nodes::{
    ChildNodesWithoutAttributes, ItemWithAttributes,
};
use crate::compiler::plugin::proc_macro::v2::host::aux_data::{EmittedAuxData, ProcMacroAuxData};
use crate::compiler::plugin::proc_macro::v2::host::conversion::{
    CallSiteLocation, into_cairo_diagnostics,
};
use crate::compiler::plugin::proc_macro::v2::host::generate_code_mappings;
use crate::compiler::plugin::proc_macro::v2::{
    ProcMacroHostPlugin, ProcMacroId, TokenStreamBuilder,
};
use cairo_lang_defs::plugin::{DynGeneratedFileAuxData, PluginGeneratedFile, PluginResult};
use cairo_lang_macro::{AllocationContext, TokenStream};
use cairo_lang_syntax::node::ast;
use cairo_lang_syntax::node::db::SyntaxGroup;
use smol_str::SmolStr;

impl ProcMacroHostPlugin {
    /// Find first attribute procedural macros that should be expanded.
    ///
    /// Remove the attribute from the code.
    pub(crate) fn parse_attribute(
        &self,
        db: &dyn SyntaxGroup,
        item_ast: ast::ModuleItem,
        ctx: &AllocationContext,
    ) -> (AttrExpansionFound, TokenStream) {
        let mut token_stream_builder = TokenStreamBuilder::new(db);
        let input = match item_ast.clone() {
            ast::ModuleItem::Trait(ast) => {
                parse_item(&ast, db, self, &mut token_stream_builder, ctx)
            }
            ast::ModuleItem::Impl(ast) => {
                parse_item(&ast, db, self, &mut token_stream_builder, ctx)
            }
            ast::ModuleItem::Module(ast) => {
                parse_item(&ast, db, self, &mut token_stream_builder, ctx)
            }
            ast::ModuleItem::FreeFunction(ast) => {
                parse_item(&ast, db, self, &mut token_stream_builder, ctx)
            }
            ast::ModuleItem::ExternFunction(ast) => {
                parse_item(&ast, db, self, &mut token_stream_builder, ctx)
            }
            ast::ModuleItem::ExternType(ast) => {
                parse_item(&ast, db, self, &mut token_stream_builder, ctx)
            }
            ast::ModuleItem::Struct(ast) => {
                parse_item(&ast, db, self, &mut token_stream_builder, ctx)
            }
            ast::ModuleItem::Enum(ast) => {
                parse_item(&ast, db, self, &mut token_stream_builder, ctx)
            }
            ast::ModuleItem::Constant(ast) => {
                parse_item(&ast, db, self, &mut token_stream_builder, ctx)
            }
            ast::ModuleItem::Use(ast) => parse_item(&ast, db, self, &mut token_stream_builder, ctx),
            ast::ModuleItem::ImplAlias(ast) => {
                parse_item(&ast, db, self, &mut token_stream_builder, ctx)
            }
            ast::ModuleItem::TypeAlias(ast) => {
                parse_item(&ast, db, self, &mut token_stream_builder, ctx)
            }
            // The items below are not supported.
            ast::ModuleItem::HeaderDoc(_) => AttrExpansionFound::None,
            ast::ModuleItem::Missing(_) => AttrExpansionFound::None,
            // TODO(#2204): Support inline macro expansion at module item level.
            ast::ModuleItem::InlineMacro(_) => AttrExpansionFound::None,
        };
        let token_stream = token_stream_builder.build(ctx);
        (input, token_stream)
    }

    pub fn expand_attribute(
        &self,
        db: &dyn SyntaxGroup,
        input: ProcMacroId,
        last: bool,
        args: TokenStream,
        token_stream: TokenStream,
        call_site: CallSiteLocation,
    ) -> PluginResult {
        let original = token_stream.to_string();
        let result = self
            .instance(input.package_id)
            .try_v2()
            .expect("procedural macro using v1 api used in a context expecting v2 api")
            .generate_code(
                input.expansion.expansion_name.clone(),
                call_site.span.clone(),
                args,
                token_stream,
            );

        // Handle token stream.
        if result.token_stream.is_empty() {
            // Remove original code
            return PluginResult {
                diagnostics: into_cairo_diagnostics(db, result.diagnostics, call_site.stable_ptr),
                code: None,
                remove_original_item: true,
            };
        }

        // Full path markers require code modification.
        self.register_full_path_markers(input.package_id, result.full_path_markers.clone());

        // This is a minor optimization.
        // If the expanded macro attribute is the only one that will be expanded by `ProcMacroHost`
        // in this `generate_code` call (i.e. all the other macro attributes has been expanded by
        // previous calls), and the expansion did not produce any changes, we can skip rewriting the
        // expanded node by simply returning no generated code, and leaving the original item as is.
        // However, if we have other macro attributes to expand, we must rewrite the node even if no
        // changes have been produced, so that we can parse the attributes once again and expand them.
        // In essence, `code: None, remove_original_item: false` means `ProcMacroHost` will not be
        // called again for this AST item.
        // This optimization limits the number of generated nodes a bit.
        if last && result.aux_data.is_none() && original == result.token_stream.to_string() {
            return PluginResult {
                code: None,
                remove_original_item: false,
                diagnostics: into_cairo_diagnostics(db, result.diagnostics, call_site.stable_ptr),
            };
        }

        let file_name = format!("proc_{}", input.expansion.cairo_name);
        let code_mappings = generate_code_mappings(&result.token_stream, call_site.span.clone());
        let content = result.token_stream.to_string();
        PluginResult {
            code: Some(PluginGeneratedFile {
                name: file_name.into(),
                code_mappings,
                content,
                diagnostics_note: Some(format!(
                    "this error originates in the attribute macro: `{}`",
                    input.expansion.cairo_name
                )),
                aux_data: result.aux_data.map(|new_aux_data| {
                    DynGeneratedFileAuxData::new(EmittedAuxData::new(ProcMacroAuxData::new(
                        new_aux_data.into(),
                        input,
                    )))
                }),
            }),
            diagnostics: into_cairo_diagnostics(db, result.diagnostics, call_site.stable_ptr),
            remove_original_item: true,
        }
    }
}

fn parse_item<T: ItemWithAttributes + ChildNodesWithoutAttributes>(
    ast: &T,
    db: &dyn SyntaxGroup,
    host: &ProcMacroHostPlugin,
    token_stream_builder: &mut TokenStreamBuilder<'_>,
    ctx: &AllocationContext,
) -> AttrExpansionFound {
    let attrs = ast.item_attributes(db);
    let expansion = host.parse_attrs(db, token_stream_builder, attrs, ctx);
    token_stream_builder.extend(ast.child_nodes_without_attributes(db));
    expansion
}

pub enum AttrExpansionFound {
    Some(AttrExpansionArgs),
    Last(AttrExpansionArgs),
    None,
}

pub struct AttrExpansionArgs {
    pub id: ProcMacroId,
    pub args: TokenStream,
    pub call_site: CallSiteLocation,
}

impl AttrExpansionFound {
    pub fn as_name(&self) -> Option<SmolStr> {
        match self {
            AttrExpansionFound::Some(args) | AttrExpansionFound::Last(args) => {
                Some(args.id.expansion.cairo_name.clone())
            }
            AttrExpansionFound::None => None,
        }
    }
}
