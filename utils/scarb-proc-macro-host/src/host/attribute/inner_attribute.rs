use std::collections::HashSet;

use cairo_lang_defs::patcher::{PatchBuilder, RewriteNode};
use cairo_lang_defs::plugin::PluginDiagnostic;
use cairo_lang_macro::{AllocationContext, ProcMacroResult, TokenStream};
use cairo_lang_syntax::node::ast::{ImplItem, MaybeImplBody, MaybeTraitBody};
use cairo_lang_syntax::node::ids::SyntaxStablePtrId;
use cairo_lang_syntax::node::{SyntaxNode, TypedSyntaxNode, ast};
use itertools::Itertools;
use salsa::Database;
use smol_str::SmolStr;

use crate::backend::{ExpansionId, ProcMacroBackend};
use crate::conversion::into_cairo_diagnostics;
use crate::host::ProcMacroHostPlugin;
use crate::host::attribute::span_adapter::{AdaptedDiagnostic, AdaptedTokenStream};
use crate::host::attribute::{
    AttrExpansionArgs, AttrExpansionFound, AttributeGeneratedFile, AttributePluginResult,
};
use crate::host::generate_code_mappings;
use crate::token_stream_builder::TokenStreamBuilder;

pub(crate) enum InnerAttrExpansionResult<'db> {
    None,
    Some(AttributePluginResult<'db>),
}

pub(crate) struct InnerAttrExpansionContext<'a, 'db, B: ProcMacroBackend> {
    host: &'a ProcMacroHostPlugin<B>,
    // Metadata returned for expansions.
    diagnostics: Vec<PluginDiagnostic<'db>>,
    aux_data: B::AuxData,
    any_changed: bool,
    item_builder: PatchBuilder<'db>,
}

impl<'a, 'db, B: ProcMacroBackend> InnerAttrExpansionContext<'a, 'db, B> {
    pub(crate) fn new(
        host: &'a ProcMacroHostPlugin<B>,
        db: &'db dyn Database,
        item_ast: &ast::ModuleItem<'db>,
    ) -> Self {
        Self {
            diagnostics: Vec::new(),
            aux_data: B::AuxData::default(),
            any_changed: false,
            item_builder: PatchBuilder::new(db, item_ast),
            host,
        }
    }

    pub(crate) fn add_node(&mut self, node: SyntaxNode<'db>) {
        self.item_builder.add_node(node);
    }

    fn register_diagnotics(
        &mut self,
        db: &'db dyn Database,
        diagnostics: Vec<AdaptedDiagnostic>,
        stable_ptr: SyntaxStablePtrId<'db>,
    ) {
        let diagnostics = diagnostics.into_iter().map(Into::into).collect();
        self.diagnostics
            .extend(into_cairo_diagnostics(db, diagnostics, stable_ptr));
    }

    pub(crate) fn register_result_metadata(
        &mut self,
        db: &'db dyn Database,
        input: &AttrExpansionArgs<'db, B::Id>,
        original: String,
        result: ProcMacroResult,
    ) {
        let result_str = result.token_stream.to_string();
        let changed = result_str != original;

        let diagnostics = input
            .attribute_location
            .adapt_diagnostics(result.diagnostics);
        self.register_diagnotics(db, diagnostics, input.call_site.stable_ptr);

        self.any_changed = self.any_changed || changed;

        self.item_builder
            .add_modified(rewrite_node_patch_from_expansion_result(
                result.token_stream,
                input,
            ));
    }

    pub(crate) fn into_result(self, attr_names: Vec<SmolStr>) -> AttributePluginResult<'db> {
        let Self {
            host,
            diagnostics,
            aux_data,
            item_builder,
            ..
        } = self;
        let aux_data = host.backend().finish_aux_data(aux_data);
        let msg = if attr_names.len() == 1 {
            "the attribute macro"
        } else {
            "one of the attribute macros"
        };
        let derive_names = attr_names.iter().map(ToString::to_string).join("`, `");
        let note = format!("this error originates in {msg}: `{derive_names}`");
        AttributePluginResult::new()
            .with_remove_original_item(true)
            .with_plugin_diagnostics(diagnostics)
            .with_generated_file(
                AttributeGeneratedFile::from_patch_builder("proc_attr_inner", item_builder)
                    .with_diagnostics_note(note)
                    .with_aux_data(aux_data),
            )
    }
}

fn rewrite_node_patch_from_expansion_result<'db, Id: ExpansionId>(
    token_stream: TokenStream,
    input: &AttrExpansionArgs<'db, Id>,
) -> RewriteNode<'db> {
    let code_mappings = generate_code_mappings(
        &token_stream,
        input.attribute_location.adapted_call_site().into(),
    );
    let code_mappings = input.attribute_location.adapt_code_mappings(code_mappings);
    let code_mappings = code_mappings.into_iter().map(Into::into).collect_vec();
    let expanded = token_stream.to_string();
    RewriteNode::TextAndMapping(expanded, code_mappings)
}

impl<B: ProcMacroBackend> ProcMacroHostPlugin<B> {
    pub(crate) fn expand_inner_attr<'db>(
        &self,
        db: &'db dyn Database,
        item_ast: ast::ModuleItem<'db>,
    ) -> InnerAttrExpansionResult<'db> {
        let mut context = InnerAttrExpansionContext::new(self, db, &item_ast);
        let mut used_attr_names: HashSet<SmolStr> = Default::default();
        let mut all_none = true;
        let ctx = AllocationContext::default();

        match item_ast.clone() {
            ast::ModuleItem::Trait(trait_ast) => {
                context.add_node(trait_ast.attributes(db).as_syntax_node());
                context.add_node(trait_ast.visibility(db).as_syntax_node());
                context.add_node(trait_ast.trait_kw(db).as_syntax_node());
                context.add_node(trait_ast.name(db).as_syntax_node());
                context.add_node(trait_ast.generic_params(db).as_syntax_node());

                // Parser attributes for inner functions.
                match trait_ast.body(db) {
                    MaybeTraitBody::None(terminal) => {
                        context.add_node(terminal.as_syntax_node());
                        InnerAttrExpansionResult::None
                    }
                    MaybeTraitBody::Some(body) => {
                        context.add_node(body.lbrace(db).as_syntax_node());

                        let item_list = body.items(db);
                        for item in item_list.elements(db) {
                            let ast::TraitItem::Function(func) = item else {
                                context.add_node(item.as_syntax_node());
                                continue;
                            };

                            let item_span = func.as_syntax_node().span(db);
                            let mut token_stream_builder = TokenStreamBuilder::new(db);
                            let attrs = func.attributes(db).elements(db).collect();
                            let found = self.parse_attrs(
                                db,
                                &mut token_stream_builder,
                                attrs,
                                item_span,
                                &ctx,
                            );
                            if let Some(name) = found.as_name() {
                                used_attr_names.insert(name);
                            }
                            token_stream_builder.add_node(func.declaration(db).as_syntax_node());
                            token_stream_builder.add_node(func.body(db).as_syntax_node());
                            let token_stream = token_stream_builder.build(&ctx);
                            let token_stream = found.adapt_token_stream(token_stream);
                            all_none = all_none
                                && self.do_expand_inner_attr(
                                    db,
                                    &mut context,
                                    found,
                                    &func,
                                    token_stream,
                                );
                        }

                        context.add_node(body.rbrace(db).as_syntax_node());

                        if all_none {
                            InnerAttrExpansionResult::None
                        } else {
                            InnerAttrExpansionResult::Some(
                                context.into_result(used_attr_names.into_iter().collect()),
                            )
                        }
                    }
                }
            }

            ast::ModuleItem::Impl(impl_ast) => {
                context.add_node(impl_ast.attributes(db).as_syntax_node());
                context.add_node(impl_ast.visibility(db).as_syntax_node());
                context.add_node(impl_ast.impl_kw(db).as_syntax_node());
                context.add_node(impl_ast.name(db).as_syntax_node());
                context.add_node(impl_ast.generic_params(db).as_syntax_node());
                context.add_node(impl_ast.of_kw(db).as_syntax_node());
                context.add_node(impl_ast.trait_path(db).as_syntax_node());

                match impl_ast.body(db) {
                    MaybeImplBody::None(terminal) => {
                        context.add_node(terminal.as_syntax_node());
                        InnerAttrExpansionResult::None
                    }
                    MaybeImplBody::Some(body) => {
                        context.add_node(body.lbrace(db).as_syntax_node());

                        let items = body.items(db);
                        for item in items.elements(db) {
                            let ImplItem::Function(func) = item else {
                                context.add_node(item.as_syntax_node());
                                continue;
                            };

                            let item_span = func.as_syntax_node().span(db);
                            let mut token_stream_builder = TokenStreamBuilder::new(db);
                            let attrs = func.attributes(db).elements(db).collect();
                            let found = self.parse_attrs(
                                db,
                                &mut token_stream_builder,
                                attrs,
                                item_span,
                                &ctx,
                            );
                            if let Some(name) = found.as_name() {
                                used_attr_names.insert(name);
                            }
                            token_stream_builder.add_node(func.visibility(db).as_syntax_node());
                            token_stream_builder.add_node(func.declaration(db).as_syntax_node());
                            token_stream_builder.add_node(func.body(db).as_syntax_node());
                            let token_stream = token_stream_builder.build(&ctx);
                            let token_stream = found.adapt_token_stream(token_stream);
                            all_none = all_none
                                && self.do_expand_inner_attr(
                                    db,
                                    &mut context,
                                    found,
                                    &func,
                                    token_stream,
                                );
                        }

                        context.add_node(body.rbrace(db).as_syntax_node());

                        if all_none {
                            InnerAttrExpansionResult::None
                        } else {
                            InnerAttrExpansionResult::Some(
                                context.into_result(used_attr_names.into_iter().collect()),
                            )
                        }
                    }
                }
            }
            _ => InnerAttrExpansionResult::None,
        }
    }

    fn do_expand_inner_attr<'a, 'db>(
        &'a self,
        db: &'db dyn Database,
        context: &mut InnerAttrExpansionContext<'a, 'db, B>,
        found: AttrExpansionFound<'db, B::Id>,
        func: &impl TypedSyntaxNode<'db>,
        token_stream: AdaptedTokenStream,
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
                context.add_node(func.as_syntax_node());
                return all_none;
            }
        };

        let result = self.expand(
            db,
            &input.id,
            input.attribute_location.adapted_call_site().into(),
            input.args.clone(),
            token_stream.clone().into(),
            &mut context.aux_data,
        );

        context.register_result_metadata(db, &input, token_stream.to_string(), result);

        all_none
    }
}
