use crate::compiler::plugin::proc_macro::v2::host::attribute::child_nodes::{
    ChildNodesWithoutAttributes, ItemWithAttributes,
};
use crate::compiler::plugin::proc_macro::v2::host::attribute::token_span;
use crate::compiler::plugin::proc_macro::v2::host::attribute::token_span::{
    TokenStreamAdaptedLocation, adapt_call_site_span,
};
use crate::compiler::plugin::proc_macro::v2::host::aux_data::{EmittedAuxData, ProcMacroAuxData};
use crate::compiler::plugin::proc_macro::v2::host::conversion::{
    CallSiteLocation, SpanSource, into_cairo_diagnostics,
};
use crate::compiler::plugin::proc_macro::v2::host::generate_code_mappings;
use crate::compiler::plugin::proc_macro::v2::{
    ProcMacroHostPlugin, ProcMacroId, TokenStreamBuilder,
};
use cairo_lang_defs::plugin::{DynGeneratedFileAuxData, PluginGeneratedFile, PluginResult};
use cairo_lang_filesystem::span::TextWidth;
use cairo_lang_macro::{AllocationContext, TextOffset, TextSpan, TokenStream};
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::{TypedSyntaxNode, ast};
use smol_str::SmolStr;

impl ProcMacroHostPlugin {
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
    /// See [`AttributeSpanAdapter`] for details.
    pub(crate) fn parse_attribute(
        &self,
        db: &dyn SyntaxGroup,
        item_ast: ast::ModuleItem,
        ctx: &AllocationContext,
    ) -> (AttrExpansionFound, TokenStreamAdaptedLocation) {
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
            // TODO(#2204): Support inline macro expansion at module item level.
            ast::ModuleItem::InlineMacro(_) => AttrExpansionFound::None,
        };
        let token_stream = token_stream_builder.build(ctx);
        let token_stream = token_span::move_spans(&input, token_stream);
        (input, token_stream)
    }

    pub fn expand_attribute(
        &self,
        db: &dyn SyntaxGroup,
        last: bool,
        args: TokenStream,
        token_stream: TokenStreamAdaptedLocation,
        input: AttrExpansionArgs,
    ) -> PluginResult {
        let original = token_stream.to_string();
        let result = self
            .instance(input.id.package_id)
            .try_v2()
            .expect("procedural macro using v1 api used in a context expecting v2 api")
            .generate_code(
                input.id.expansion.expansion_name.clone(),
                input.attribute_location.adapted_call_site.clone(),
                args,
                token_stream.into(),
            );

        // Handle token stream.
        if result.token_stream.is_empty() {
            // Remove original code
            return PluginResult {
                diagnostics: into_cairo_diagnostics(
                    db,
                    result.diagnostics,
                    input.call_site.stable_ptr,
                ),
                code: None,
                remove_original_item: true,
            };
        }

        // Full path markers require code modification.
        self.register_full_path_markers(input.id.package_id, result.full_path_markers.clone());

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
                diagnostics: into_cairo_diagnostics(
                    db,
                    result.diagnostics,
                    input.call_site.stable_ptr,
                ),
            };
        }

        let file_name = format!("proc_{}", input.id.expansion.cairo_name);
        let code_mappings =
            generate_code_mappings(&result.token_stream, input.call_site.span.clone());
        let code_mappings =
            token_span::move_mappings_by_expanded_attr(code_mappings, input.attribute_location);
        let content = result.token_stream.to_string();
        PluginResult {
            code: Some(PluginGeneratedFile {
                name: file_name.into(),
                code_mappings,
                content,
                diagnostics_note: Some(format!(
                    "this error originates in the attribute macro: `{}`",
                    input.id.expansion.cairo_name
                )),
                aux_data: result.aux_data.map(|new_aux_data| {
                    DynGeneratedFileAuxData::new(EmittedAuxData::new(ProcMacroAuxData::new(
                        new_aux_data.into(),
                        input.id,
                    )))
                }),
            }),
            diagnostics: into_cairo_diagnostics(db, result.diagnostics, input.call_site.stable_ptr),
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
    pub attribute_location: ExpandableAttrLocation,
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

pub struct ExpandableAttrLocation {
    pub token_offset: TextOffset,
    pub token_length: TextWidth,
    pub adapted_call_site: TextSpan,
}

impl ExpandableAttrLocation {
    pub fn new<T: TypedSyntaxNode>(node: &T, db: &dyn SyntaxGroup) -> Self {
        let text_span = node.text_span(db);
        let token_length = text_span.end - text_span.start;
        Self {
            token_offset: text_span.start,
            token_length: TextWidth::new_for_testing(token_length + 1),
            adapted_call_site: adapt_call_site_span(node.text_span(db), text_span.start),
        }
    }
}
