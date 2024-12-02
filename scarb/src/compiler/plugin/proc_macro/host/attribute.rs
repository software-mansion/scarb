use crate::compiler::plugin::proc_macro::host::aux_data::{EmittedAuxData, ProcMacroAuxData};
use crate::compiler::plugin::proc_macro::host::into_cairo_diagnostics;
use crate::compiler::plugin::proc_macro::{
    Expansion, ExpansionKind, ProcMacroHostPlugin, ProcMacroId, TokenStreamBuilder,
};
use cairo_lang_defs::patcher::{PatchBuilder, RewriteNode};
use cairo_lang_defs::plugin::PluginDiagnostic;
use cairo_lang_defs::plugin::{DynGeneratedFileAuxData, PluginGeneratedFile, PluginResult};
use cairo_lang_filesystem::ids::CodeMapping;
use cairo_lang_macro::{AllocationContext, ProcMacroResult, TokenStream};
use cairo_lang_syntax::attribute::structured::AttributeStructurize;
use cairo_lang_syntax::node::ast::{ImplItem, MaybeImplBody, MaybeTraitBody};
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::ids::SyntaxStablePtrId;
use cairo_lang_syntax::node::{ast, TypedStablePtr, TypedSyntaxNode};
use itertools::Itertools;
use smol_str::SmolStr;
use std::collections::HashSet;

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
                            let (code, mappings) = item_builder.build();
                            InnerAttrExpansionResult::Some(context.into_result(
                                code,
                                mappings,
                                used_attr_names.into_iter().collect(),
                            ))
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
                            let (code, mappings) = item_builder.build();
                            InnerAttrExpansionResult::Some(context.into_result(
                                code,
                                mappings,
                                used_attr_names.into_iter().collect(),
                            ))
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
        token_stream: TokenStream,
    ) -> bool {
        let mut all_none = true;
        let (input, args, stable_ptr) = match found {
            AttrExpansionFound::Last {
                expansion,
                args,
                stable_ptr,
            } => {
                all_none = false;
                (expansion, args, stable_ptr)
            }
            AttrExpansionFound::Some {
                expansion,
                args,
                stable_ptr,
            } => {
                all_none = false;
                (expansion, args, stable_ptr)
            }
            AttrExpansionFound::None => {
                item_builder.add_node(func.as_syntax_node());
                return all_none;
            }
        };

        let result = self.instance(input.package_id).generate_code(
            input.expansion.name.clone(),
            args,
            token_stream.clone(),
        );

        let expanded = context.register_result(token_stream.to_string(), input, result, stable_ptr);
        item_builder.add_modified(RewriteNode::Mapped {
            origin: func.as_syntax_node().span(db),
            node: Box::new(RewriteNode::Text(expanded.to_string())),
        });

        all_none
    }

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
            ast::ModuleItem::Trait(trait_ast) => {
                let attrs = trait_ast.attributes(db).elements(db);
                let expansion = self.parse_attrs(db, &mut token_stream_builder, attrs, ctx);
                token_stream_builder.add_node(trait_ast.visibility(db).as_syntax_node());
                token_stream_builder.add_node(trait_ast.trait_kw(db).as_syntax_node());
                token_stream_builder.add_node(trait_ast.name(db).as_syntax_node());
                token_stream_builder.add_node(trait_ast.generic_params(db).as_syntax_node());
                token_stream_builder.add_node(trait_ast.body(db).as_syntax_node());
                expansion
            }
            ast::ModuleItem::Impl(impl_ast) => {
                let attrs = impl_ast.attributes(db).elements(db);
                let expansion = self.parse_attrs(db, &mut token_stream_builder, attrs, ctx);
                token_stream_builder.add_node(impl_ast.visibility(db).as_syntax_node());
                token_stream_builder.add_node(impl_ast.impl_kw(db).as_syntax_node());
                token_stream_builder.add_node(impl_ast.name(db).as_syntax_node());
                token_stream_builder.add_node(impl_ast.generic_params(db).as_syntax_node());
                token_stream_builder.add_node(impl_ast.of_kw(db).as_syntax_node());
                token_stream_builder.add_node(impl_ast.trait_path(db).as_syntax_node());
                token_stream_builder.add_node(impl_ast.body(db).as_syntax_node());
                expansion
            }
            ast::ModuleItem::Module(module_ast) => {
                let attrs = module_ast.attributes(db).elements(db);
                let expansion = self.parse_attrs(db, &mut token_stream_builder, attrs, ctx);
                token_stream_builder.add_node(module_ast.visibility(db).as_syntax_node());
                token_stream_builder.add_node(module_ast.module_kw(db).as_syntax_node());
                token_stream_builder.add_node(module_ast.name(db).as_syntax_node());
                token_stream_builder.add_node(module_ast.body(db).as_syntax_node());
                expansion
            }
            ast::ModuleItem::FreeFunction(free_func_ast) => {
                let attrs = free_func_ast.attributes(db).elements(db);
                let expansion = self.parse_attrs(db, &mut token_stream_builder, attrs, ctx);
                token_stream_builder.add_node(free_func_ast.visibility(db).as_syntax_node());
                token_stream_builder.add_node(free_func_ast.declaration(db).as_syntax_node());
                token_stream_builder.add_node(free_func_ast.body(db).as_syntax_node());
                expansion
            }
            ast::ModuleItem::ExternFunction(extern_func_ast) => {
                let attrs = extern_func_ast.attributes(db).elements(db);
                let expansion = self.parse_attrs(db, &mut token_stream_builder, attrs, ctx);
                token_stream_builder.add_node(extern_func_ast.visibility(db).as_syntax_node());
                token_stream_builder.add_node(extern_func_ast.extern_kw(db).as_syntax_node());
                token_stream_builder.add_node(extern_func_ast.declaration(db).as_syntax_node());
                token_stream_builder.add_node(extern_func_ast.semicolon(db).as_syntax_node());
                expansion
            }
            ast::ModuleItem::ExternType(extern_type_ast) => {
                let attrs = extern_type_ast.attributes(db).elements(db);
                let expansion = self.parse_attrs(db, &mut token_stream_builder, attrs, ctx);
                token_stream_builder.add_node(extern_type_ast.visibility(db).as_syntax_node());
                token_stream_builder.add_node(extern_type_ast.extern_kw(db).as_syntax_node());
                token_stream_builder.add_node(extern_type_ast.type_kw(db).as_syntax_node());
                token_stream_builder.add_node(extern_type_ast.name(db).as_syntax_node());
                token_stream_builder.add_node(extern_type_ast.generic_params(db).as_syntax_node());
                token_stream_builder.add_node(extern_type_ast.semicolon(db).as_syntax_node());
                expansion
            }
            ast::ModuleItem::Struct(struct_ast) => {
                let attrs = struct_ast.attributes(db).elements(db);
                let expansion = self.parse_attrs(db, &mut token_stream_builder, attrs, ctx);
                token_stream_builder.add_node(struct_ast.visibility(db).as_syntax_node());
                token_stream_builder.add_node(struct_ast.struct_kw(db).as_syntax_node());
                token_stream_builder.add_node(struct_ast.name(db).as_syntax_node());
                token_stream_builder.add_node(struct_ast.generic_params(db).as_syntax_node());
                token_stream_builder.add_node(struct_ast.lbrace(db).as_syntax_node());
                token_stream_builder.add_node(struct_ast.members(db).as_syntax_node());
                token_stream_builder.add_node(struct_ast.rbrace(db).as_syntax_node());
                expansion
            }
            ast::ModuleItem::Enum(enum_ast) => {
                let attrs = enum_ast.attributes(db).elements(db);
                let expansion = self.parse_attrs(db, &mut token_stream_builder, attrs, ctx);
                token_stream_builder.add_node(enum_ast.visibility(db).as_syntax_node());
                token_stream_builder.add_node(enum_ast.enum_kw(db).as_syntax_node());
                token_stream_builder.add_node(enum_ast.name(db).as_syntax_node());
                token_stream_builder.add_node(enum_ast.generic_params(db).as_syntax_node());
                token_stream_builder.add_node(enum_ast.lbrace(db).as_syntax_node());
                token_stream_builder.add_node(enum_ast.variants(db).as_syntax_node());
                token_stream_builder.add_node(enum_ast.rbrace(db).as_syntax_node());
                expansion
            }
            _ => AttrExpansionFound::None,
        };
        let token_stream = token_stream_builder.build(ctx);
        (input, token_stream)
    }

    fn parse_attrs(
        &self,
        db: &dyn SyntaxGroup,
        builder: &mut TokenStreamBuilder<'_>,
        attrs: Vec<ast::Attribute>,
        ctx: &AllocationContext,
    ) -> AttrExpansionFound {
        // This function parses attributes of the item,
        // checking if those attributes correspond to a procedural macro that should be fired.
        // The proc macro attribute found is removed from attributes list,
        // while other attributes are appended to the `PathBuilder` passed as an argument.

        // Note this function does not affect the executable attributes,
        // as it only pulls `ExpansionKind::Attr` from the plugin.
        // This means that executable attributes will neither be removed from the item,
        // nor will they cause the item to be rewritten.
        let mut expansion = None;
        let mut last = true;
        for attr in attrs {
            // We ensure that this flag is changed *after* the expansion is found.
            if last {
                let structured_attr = attr.clone().structurize(db);
                let found = self.find_expansion(&Expansion::new(
                    structured_attr.id.clone(),
                    ExpansionKind::Attr,
                ));
                if let Some(found) = found {
                    if expansion.is_none() {
                        let mut args_builder = TokenStreamBuilder::new(db);
                        args_builder.add_node(attr.arguments(db).as_syntax_node());
                        let args = args_builder.build(ctx);
                        expansion = Some((found, args, attr.stable_ptr().untyped()));
                        // Do not add the attribute for found expansion.
                        continue;
                    } else {
                        last = false;
                    }
                }
            }
            builder.add_node(attr.as_syntax_node());
        }
        match (expansion, last) {
            (Some((expansion, args, stable_ptr)), true) => AttrExpansionFound::Last {
                expansion,
                args,
                stable_ptr,
            },
            (Some((expansion, args, stable_ptr)), false) => AttrExpansionFound::Some {
                expansion,
                args,
                stable_ptr,
            },
            (None, _) => AttrExpansionFound::None,
        }
    }

    pub fn expand_attribute(
        &self,
        input: ProcMacroId,
        last: bool,
        args: TokenStream,
        token_stream: TokenStream,
        stable_ptr: SyntaxStablePtrId,
    ) -> PluginResult {
        let original = token_stream.to_string();
        let result = self.instance(input.package_id).generate_code(
            input.expansion.name.clone(),
            args,
            token_stream,
        );

        // Handle token stream.
        if result.token_stream.is_empty() {
            // Remove original code
            return PluginResult {
                diagnostics: into_cairo_diagnostics(result.diagnostics, stable_ptr),
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
                diagnostics: into_cairo_diagnostics(result.diagnostics, stable_ptr),
            };
        }

        let file_name = format!("proc_{}", input.expansion.name);
        let content = result.token_stream.to_string();
        PluginResult {
            code: Some(PluginGeneratedFile {
                name: file_name.into(),
                code_mappings: Vec::new(),
                content,
                diagnostics_note: Some(format!(
                    "this error originates in the attribute macro: `{}`",
                    input.expansion.name
                )),
                aux_data: result.aux_data.map(|new_aux_data| {
                    DynGeneratedFileAuxData::new(EmittedAuxData::new(ProcMacroAuxData::new(
                        new_aux_data.into(),
                        input,
                    )))
                }),
            }),
            diagnostics: into_cairo_diagnostics(result.diagnostics, stable_ptr),
            remove_original_item: true,
        }
    }
}

pub enum AttrExpansionFound {
    Some {
        expansion: ProcMacroId,
        args: TokenStream,
        stable_ptr: SyntaxStablePtrId,
    },
    None,
    Last {
        expansion: ProcMacroId,
        args: TokenStream,
        stable_ptr: SyntaxStablePtrId,
    },
}

impl AttrExpansionFound {
    pub fn as_name(&self) -> Option<SmolStr> {
        match self {
            AttrExpansionFound::Some { expansion, .. }
            | AttrExpansionFound::Last { expansion, .. } => Some(expansion.expansion.name.clone()),
            AttrExpansionFound::None => None,
        }
    }
}

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
        original: String,
        input: ProcMacroId,
        result: ProcMacroResult,
        stable_ptr: SyntaxStablePtrId,
    ) -> String {
        let result_str = result.token_stream.to_string();
        let changed = result_str != original;

        if changed {
            self.host
                .register_full_path_markers(input.package_id, result.full_path_markers.clone());
        }

        self.diagnostics
            .extend(into_cairo_diagnostics(result.diagnostics, stable_ptr));

        if let Some(new_aux_data) = result.aux_data {
            self.aux_data
                .push(ProcMacroAuxData::new(new_aux_data.into(), input));
        }

        self.any_changed = self.any_changed || changed;

        result_str
    }

    pub fn into_result(
        self,
        expanded: String,
        code_mappings: Vec<CodeMapping>,
        attr_names: Vec<SmolStr>,
    ) -> PluginResult {
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
