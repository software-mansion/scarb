use cairo_lang_macro::{AllocationContext, TokenStream};
use cairo_lang_syntax::node::ast;
use salsa::Database;
use smol_str::SmolStr;

use crate::backend::{ExpansionId, ProcMacroBackend};
use crate::host::ProcMacroHostPlugin;
use crate::host::attribute::child_nodes::{ChildNodesWithoutAttributes, ItemWithAttributes};
use crate::host::attribute::span_adapter::{AdaptedTokenStream, ExpandableAttrLocation};
use crate::host::attribute::{AttributeGeneratedFile, AttributePluginResult};
use crate::host::generate_code_mappings;
use crate::token_stream_builder::TokenStreamBuilder;
use crate::conversion::CallSiteLocation;

impl<B: ProcMacroBackend> ProcMacroHostPlugin<B> {
    /// Find first attribute procedural macro that should be expanded.
    ///
    /// This method serves two purposes:
    /// 1. Parse the attributes of the module item, looking for attributes that can be expanded by
    ///    this procedural macro host.
    /// 2. Construct a `TokenStream` that can be used as input for the macro expansion.
    ///
    /// The attributes are searched in the order they appear in the code, from top to bottom.
    /// If an attribute representing an expandable procedural macro is found,
    /// it's removed from the attributes list and returned in `AttrExpansionFound`.
    /// The remaining attributes and body of the module item are concatenated into a `TokenStream`.
    ///
    /// This poses a problem, as procedural macro implementation must assume that the `TokenStream`
    /// provided as an input is consecutive. This limitation comes from how the token stream parser
    /// works, as it only sets the initial offset, and generates the rest of the spans as it would
    /// when parsing a source file. Obviously, when we remove the attribute from the `TokenStream`
    /// built, it's no longer consecutive.
    ///
    /// See [`crate::host::attribute::span_adapter`] for details.
    pub(crate) fn parse_attribute<'db>(
        &self,
        db: &'db dyn Database,
        item_ast: ast::ModuleItem<'db>,
        ctx: &AllocationContext,
    ) -> (AttrExpansionFound<'db, B::Id>, AdaptedTokenStream) {
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
            ast::ModuleItem::MacroDeclaration(_) => AttrExpansionFound::None,
            ast::ModuleItem::InlineMacro(_) => AttrExpansionFound::None,
        };
        let token_stream = input.adapt_token_stream(token_stream_builder.build(ctx));
        (input, token_stream)
    }

    pub(crate) fn expand_attribute<'db>(
        &self,
        db: &'db dyn Database,
        last: bool,
        args: TokenStream,
        token_stream: AdaptedTokenStream,
        input: AttrExpansionArgs<'db, B::Id>,
    ) -> AttributePluginResult<'db> {
        let original = token_stream.to_string();
        let mut aux_data = B::AuxData::default();
        let result = self.expand(
            db,
            &input.id,
            input.attribute_location.adapted_call_site().into(),
            args,
            token_stream.into(),
            &mut aux_data,
        );

        // Handle token stream.
        if result.token_stream.is_empty() {
            // Remove original code
            return AttributePluginResult::new()
                .with_remove_original_item(true)
                .with_diagnostics(
                    db,
                    input.call_site.stable_ptr,
                    input
                        .attribute_location
                        .adapt_diagnostics(result.diagnostics),
                );
        }

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
            return AttributePluginResult::new().with_diagnostics(
                db,
                input.call_site.stable_ptr,
                input
                    .attribute_location
                    .adapt_diagnostics(result.diagnostics),
            );
        }

        let file_name = format!("proc_{}", input.id.expansion().cairo_name);
        let code_mappings = generate_code_mappings(
            &result.token_stream,
            input.attribute_location.adapted_call_site().into(),
        );
        let code_mappings = input.attribute_location.adapt_code_mappings(code_mappings);
        let content = result.token_stream.to_string();

        AttributePluginResult::new()
            .with_remove_original_item(true)
            .with_diagnostics(
                db,
                input.call_site.stable_ptr,
                input
                    .attribute_location
                    .adapt_diagnostics(result.diagnostics),
            )
            .with_generated_file(
                AttributeGeneratedFile::new(file_name)
                    .with_content(content)
                    .with_code_mappings(code_mappings)
                    .with_aux_data(self.backend().finish_aux_data(aux_data))
                    .with_diagnostics_note(format!(
                        "this error originates in the attribute macro: `{}`",
                        input.id.expansion().cairo_name
                    )),
            )
    }
}

fn parse_item<'db, T: ItemWithAttributes<'db> + ChildNodesWithoutAttributes<'db>, B>(
    ast: &T,
    db: &'db dyn Database,
    host: &ProcMacroHostPlugin<B>,
    token_stream_builder: &mut TokenStreamBuilder<'db>,
    ctx: &AllocationContext,
) -> AttrExpansionFound<'db, B::Id>
where
    B: ProcMacroBackend,
{
    let span = ast.span_with_trivia(db);
    let attrs = ast.item_attributes(db);
    let expansion = host.parse_attrs(db, token_stream_builder, attrs, span, ctx);
    token_stream_builder.extend(ast.child_nodes_without_attributes(db));
    expansion
}

pub(crate) enum AttrExpansionFound<'db, Id: ExpansionId> {
    Some(AttrExpansionArgs<'db, Id>),
    Last(AttrExpansionArgs<'db, Id>),
    None,
}

pub(crate) struct AttrExpansionArgs<'db, Id: ExpansionId> {
    pub id: Id,
    pub args: TokenStream,
    pub call_site: CallSiteLocation<'db>,
    pub attribute_location: ExpandableAttrLocation,
}

impl<'db, Id: ExpansionId> AttrExpansionFound<'db, Id> {
    pub(crate) fn as_name(&self) -> Option<SmolStr> {
        match self {
            AttrExpansionFound::Some(args) | AttrExpansionFound::Last(args) => {
                Some(args.id.expansion().cairo_name.clone())
            }
            AttrExpansionFound::None => None,
        }
    }
}
