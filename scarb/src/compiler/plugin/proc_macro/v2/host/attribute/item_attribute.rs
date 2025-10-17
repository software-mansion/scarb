use crate::compiler::plugin::proc_macro::v2::host::attribute::child_nodes::{
    ChildNodesWithoutAttributes, ItemWithAttributes,
};
use crate::compiler::plugin::proc_macro::v2::host::attribute::span_adapter::{
    AdaptedTextSpan, AdaptedTokenStream, ExpandableAttrLocation,
};
use crate::compiler::plugin::proc_macro::v2::host::attribute::{
    AttributeGeneratedFile, AttributePluginResult,
};
use crate::compiler::plugin::proc_macro::v2::host::aux_data::EmittedAuxData;
use crate::compiler::plugin::proc_macro::v2::host::conversion::CallSiteLocation;
use crate::compiler::plugin::proc_macro::v2::host::generate_code_mappings;
use crate::compiler::plugin::proc_macro::v2::{
    ProcMacroAuxData, ProcMacroHostPlugin, ProcMacroId, TokenStreamBuilder,
};
use crate::core::PackageId;
use cairo_lang_macro::{AllocationContext, ProcMacroResult, TokenStream};
use cairo_lang_syntax::node::ast;
use salsa::Database;
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
    pub(crate) fn parse_attribute<'db>(
        &self,
        db: &'db dyn Database,
        item_ast: ast::ModuleItem<'db>,
        ctx: &AllocationContext,
    ) -> (AttrExpansionFound<'db>, AdaptedTokenStream) {
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

    pub(crate) fn generate_attribute_code(
        &self,
        package_id: PackageId,
        item_name: SmolStr,
        call_site: AdaptedTextSpan,
        attr: TokenStream,
        token_stream: AdaptedTokenStream,
    ) -> ProcMacroResult {
        self.instance(package_id)
            .try_v2()
            .expect("procedural macro using v1 api used in a context expecting v2 api")
            .generate_code(item_name, call_site.into(), attr, token_stream.into())
    }

    pub fn expand_attribute<'db>(
        &self,
        db: &'db dyn Database,
        last: bool,
        args: TokenStream,
        token_stream: AdaptedTokenStream,
        input: AttrExpansionArgs<'db>,
    ) -> AttributePluginResult<'db> {
        let original = token_stream.to_string();
        let result = self.generate_attribute_code(
            input.id.package_id,
            input.id.expansion.expansion_name.clone(),
            input.attribute_location.adapted_call_site(),
            args,
            token_stream,
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
            return AttributePluginResult::new().with_diagnostics(
                db,
                input.call_site.stable_ptr,
                input
                    .attribute_location
                    .adapt_diagnostics(result.diagnostics),
            );
        }

        let file_name = format!("proc_{}", input.id.expansion.cairo_name);
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
                    .with_aux_data(
                        result
                            .aux_data
                            .map(|new_aux_data| {
                                EmittedAuxData::new(ProcMacroAuxData::new(
                                    new_aux_data.into(),
                                    input.id.clone(),
                                ))
                            })
                            .unwrap_or_default(),
                    )
                    .with_diagnostics_note(format!(
                        "this error originates in the attribute macro: `{}`",
                        input.id.expansion.cairo_name
                    )),
            )
    }
}

fn parse_item<'db, T: ItemWithAttributes<'db> + ChildNodesWithoutAttributes<'db>>(
    ast: &T,
    db: &'db dyn Database,
    host: &ProcMacroHostPlugin,
    token_stream_builder: &mut TokenStreamBuilder<'db>,
    ctx: &AllocationContext,
) -> AttrExpansionFound<'db> {
    let span = ast.span_with_trivia(db);
    let attrs = ast.item_attributes(db);
    let expansion = host.parse_attrs(db, token_stream_builder, attrs, span, ctx);
    token_stream_builder.extend(ast.child_nodes_without_attributes(db));
    expansion
}

pub enum AttrExpansionFound<'db> {
    Some(AttrExpansionArgs<'db>),
    Last(AttrExpansionArgs<'db>),
    None,
}

pub struct AttrExpansionArgs<'db> {
    pub id: ProcMacroId,
    pub args: TokenStream,
    pub call_site: CallSiteLocation<'db>,
    pub attribute_location: ExpandableAttrLocation,
}

impl<'db> AttrExpansionFound<'db> {
    pub fn as_name(&self) -> Option<SmolStr> {
        match self {
            AttrExpansionFound::Some(args) | AttrExpansionFound::Last(args) => {
                Some(args.id.expansion.cairo_name.clone())
            }
            AttrExpansionFound::None => None,
        }
    }
}
