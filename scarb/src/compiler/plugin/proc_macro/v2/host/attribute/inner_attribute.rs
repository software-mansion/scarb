use crate::compiler::plugin::proc_macro::v2::host::attribute::token_span::TokenStreamAdaptedLocation;
use crate::compiler::plugin::proc_macro::v2::host::attribute::{
    AttrExpansionFound, ExpandableAttrLocation, token_span,
};
use crate::compiler::plugin::proc_macro::v2::host::aux_data::EmittedAuxData;
use crate::compiler::plugin::proc_macro::v2::host::conversion::into_cairo_diagnostics;
use crate::compiler::plugin::proc_macro::v2::{
    ProcMacroAuxData, ProcMacroHostPlugin, ProcMacroId, TokenStreamBuilder, generate_code_mappings,
};
use cairo_lang_defs::patcher::{PatchBuilder, RewriteNode};
use cairo_lang_defs::plugin::{
    DynGeneratedFileAuxData, PluginDiagnostic, PluginGeneratedFile, PluginResult,
};
use cairo_lang_macro::{AllocationContext, ProcMacroResult};
use cairo_lang_syntax::node::ast::{ImplItem, MaybeImplBody, MaybeTraitBody};
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::ids::SyntaxStablePtrId;
use cairo_lang_syntax::node::{TypedSyntaxNode, ast};
use itertools::Itertools;
use smol_str::SmolStr;
use std::collections::HashSet;

pub enum InnerAttrExpansionResult {
    None,
    Some(PluginResult),
}

pub struct InnerAttrExpansionContext<'a> {
    host: &'a ProcMacroHostPlugin,
    // Metadata returned for expansions.
    diagnostics: Vec<PluginDiagnostic>,
    aux_data: EmittedAuxData,
    any_changed: bool,
}

impl<'a> InnerAttrExpansionContext<'a> {
    pub fn new<'b: 'a>(host: &'b ProcMacroHostPlugin) -> Self {
        Self {
            diagnostics: Vec::new(),
            aux_data: EmittedAuxData::default(),
            any_changed: false,
            host,
        }
    }

    pub fn register_result(
        &mut self,
        db: &dyn SyntaxGroup,
        original: String,
        input: ProcMacroId,
        result: ProcMacroResult,
        stable_ptr: SyntaxStablePtrId,
        attribute_span: &ExpandableAttrLocation,
    ) -> String {
        let result_str = result.token_stream.to_string();
        let changed = result_str != original;

        if changed {
            self.host
                .register_full_path_markers(input.package_id, result.full_path_markers.clone());
        }

        let diagnostics =
            token_span::move_diagnostics_span_by_expanded_attr(result.diagnostics, attribute_span);
        self.diagnostics
            .extend(into_cairo_diagnostics(db, diagnostics, stable_ptr));

        if let Some(new_aux_data) = result.aux_data {
            self.aux_data
                .push(ProcMacroAuxData::new(new_aux_data.into(), input));
        }

        self.any_changed = self.any_changed || changed;

        result_str
    }

    pub fn into_result(
        self,
        item_builder: PatchBuilder<'_>,
        attr_names: Vec<SmolStr>,
    ) -> PluginResult {
        let (expanded, mut code_mappings) = item_builder.build();
        // PatchBuilder::build() adds additional mapping at the end,
        // which wraps the whole outputted code.
        // We remove it, so we can properly translate locations spanning multiple token spans.
        code_mappings.pop();
        let msg = if attr_names.len() == 1 {
            "the attribute macro"
        } else {
            "one of the attribute macros"
        };
        let derive_names = attr_names.iter().map(ToString::to_string).join("`, `");
        let note = format!("this error originates in {msg}: `{derive_names}`");
        PluginResult {
            code: Some(PluginGeneratedFile {
                name: "proc_attr_inner".into(),
                content: expanded,
                aux_data: if self.aux_data.is_empty() {
                    None
                } else {
                    Some(DynGeneratedFileAuxData::new(self.aux_data))
                },
                code_mappings,
                diagnostics_note: Some(note),
            }),
            diagnostics: self.diagnostics,
            remove_original_item: true,
        }
    }
}

impl ProcMacroHostPlugin {
    pub(crate) fn expand_inner_attr(
        &self,
        db: &dyn SyntaxGroup,
        item_ast: ast::ModuleItem,
    ) -> InnerAttrExpansionResult {
        let mut context = InnerAttrExpansionContext::new(self);
        let mut item_builder = PatchBuilder::new(db, &item_ast);
        let mut used_attr_names: HashSet<SmolStr> = Default::default();
        let mut all_none = true;
        let ctx = AllocationContext::default();

        match item_ast.clone() {
            ast::ModuleItem::Trait(trait_ast) => {
                item_builder.add_node(trait_ast.attributes(db).as_syntax_node());
                item_builder.add_node(trait_ast.visibility(db).as_syntax_node());
                item_builder.add_node(trait_ast.trait_kw(db).as_syntax_node());
                item_builder.add_node(trait_ast.name(db).as_syntax_node());
                item_builder.add_node(trait_ast.generic_params(db).as_syntax_node());

                // Parser attributes for inner functions.
                match trait_ast.body(db) {
                    MaybeTraitBody::None(terminal) => {
                        item_builder.add_node(terminal.as_syntax_node());
                        InnerAttrExpansionResult::None
                    }
                    MaybeTraitBody::Some(body) => {
                        item_builder.add_node(body.lbrace(db).as_syntax_node());

                        let item_list = body.items(db);
                        for item in item_list.elements(db).iter() {
                            let ast::TraitItem::Function(func) = item else {
                                item_builder.add_node(item.as_syntax_node());
                                continue;
                            };

                            let mut token_stream_builder = TokenStreamBuilder::new(db);
                            let attrs = func.attributes(db).elements(db);
                            let found =
                                self.parse_attrs(db, &mut token_stream_builder, attrs, &ctx);
                            if let Some(name) = found.as_name() {
                                used_attr_names.insert(name);
                            }
                            token_stream_builder.add_node(func.declaration(db).as_syntax_node());
                            token_stream_builder.add_node(func.body(db).as_syntax_node());
                            let token_stream = token_stream_builder.build(&ctx);
                            let token_stream = token_span::move_spans(&found, token_stream);
                            all_none = all_none
                                && self.do_expand_inner_attr(
                                    db,
                                    &mut context,
                                    &mut item_builder,
                                    found,
                                    func,
                                    token_stream,
                                );
                        }

                        item_builder.add_node(body.rbrace(db).as_syntax_node());

                        if all_none {
                            InnerAttrExpansionResult::None
                        } else {
                            InnerAttrExpansionResult::Some(
                                context.into_result(
                                    item_builder,
                                    used_attr_names.into_iter().collect(),
                                ),
                            )
                        }
                    }
                }
            }

            ast::ModuleItem::Impl(impl_ast) => {
                item_builder.add_node(impl_ast.attributes(db).as_syntax_node());
                item_builder.add_node(impl_ast.visibility(db).as_syntax_node());
                item_builder.add_node(impl_ast.impl_kw(db).as_syntax_node());
                item_builder.add_node(impl_ast.name(db).as_syntax_node());
                item_builder.add_node(impl_ast.generic_params(db).as_syntax_node());
                item_builder.add_node(impl_ast.of_kw(db).as_syntax_node());
                item_builder.add_node(impl_ast.trait_path(db).as_syntax_node());

                match impl_ast.body(db) {
                    MaybeImplBody::None(terminal) => {
                        item_builder.add_node(terminal.as_syntax_node());
                        InnerAttrExpansionResult::None
                    }
                    MaybeImplBody::Some(body) => {
                        item_builder.add_node(body.lbrace(db).as_syntax_node());

                        let items = body.items(db);
                        for item in items.elements(db) {
                            let ImplItem::Function(func) = item else {
                                item_builder.add_node(item.as_syntax_node());
                                continue;
                            };

                            let mut token_stream_builder = TokenStreamBuilder::new(db);
                            let attrs = func.attributes(db).elements(db);
                            let found =
                                self.parse_attrs(db, &mut token_stream_builder, attrs, &ctx);
                            if let Some(name) = found.as_name() {
                                used_attr_names.insert(name);
                            }
                            token_stream_builder.add_node(func.visibility(db).as_syntax_node());
                            token_stream_builder.add_node(func.declaration(db).as_syntax_node());
                            token_stream_builder.add_node(func.body(db).as_syntax_node());
                            let token_stream = token_stream_builder.build(&ctx);
                            let token_stream = token_span::move_spans(&found, token_stream);
                            all_none = all_none
                                && self.do_expand_inner_attr(
                                    db,
                                    &mut context,
                                    &mut item_builder,
                                    found,
                                    &func,
                                    token_stream,
                                );
                        }

                        item_builder.add_node(body.rbrace(db).as_syntax_node());

                        if all_none {
                            InnerAttrExpansionResult::None
                        } else {
                            InnerAttrExpansionResult::Some(
                                context.into_result(
                                    item_builder,
                                    used_attr_names.into_iter().collect(),
                                ),
                            )
                        }
                    }
                }
            }
            _ => InnerAttrExpansionResult::None,
        }
    }

    fn do_expand_inner_attr(
        &self,
        db: &dyn SyntaxGroup,
        context: &mut InnerAttrExpansionContext<'_>,
        item_builder: &mut PatchBuilder<'_>,
        found: AttrExpansionFound,
        func: &impl TypedSyntaxNode,
        token_stream: TokenStreamAdaptedLocation,
    ) -> bool {
        let mut all_none = true;
        let input = match found {
            AttrExpansionFound::Last(input) => {
                all_none = false;
                input
            }
            AttrExpansionFound::Some(input) => {
                all_none = false;
                input
            }
            AttrExpansionFound::None => {
                item_builder.add_node(func.as_syntax_node());
                return all_none;
            }
        };

        let result = self
            .instance(input.id.package_id)
            .try_v2()
            .expect("procedural macro using v1 api used in a context expecting v2 api")
            .generate_code(
                input.id.expansion.expansion_name.clone(),
                input.attribute_location.adapted_call_site.clone(),
                input.args,
                token_stream.clone().into(),
            );

        let code_mappings =
            generate_code_mappings(&result.token_stream, input.call_site.span.clone());
        let code_mappings =
            token_span::move_mappings_by_expanded_attr(code_mappings, &input.attribute_location);
        let expanded = context.register_result(
            db,
            token_stream.to_string(),
            input.id,
            result,
            input.call_site.stable_ptr,
            &input.attribute_location,
        );
        item_builder.add_modified(RewriteNode::TextAndMapping(expanded, code_mappings));

        all_none
    }
}
