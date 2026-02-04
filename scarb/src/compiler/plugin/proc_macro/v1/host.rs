use crate::compiler::plugin::proc_macro::expansion::{Expansion, ExpansionKind};
use crate::compiler::plugin::proc_macro::v1::FromSyntaxNode;
use crate::compiler::plugin::proc_macro::{
    DeclaredProcMacroInstances, ExpansionQuery, FULL_PATH_MARKER_KEY, ProcMacroInstance,
};
use crate::core::PackageId;
use anyhow::{Result, ensure};
use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::ids::{ModuleItemId, TopLevelLanguageElementId};
use cairo_lang_defs::patcher::{PatchBuilder, RewriteNode};
use cairo_lang_defs::plugin::{
    DynGeneratedFileAuxData, GeneratedFileAuxData, MacroPlugin, MacroPluginMetadata,
    PluginGeneratedFile, PluginResult,
};
use cairo_lang_defs::plugin::{InlineMacroExprPlugin, InlinePluginResult, PluginDiagnostic};
use cairo_lang_diagnostics::ToOption;
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::{CodeMapping, SmolStrId};
use cairo_lang_macro_v1::{
    AuxData, Diagnostic, FullPathMarker, ProcMacroResult, Severity, TokenStream,
    TokenStreamMetadata,
};
use cairo_lang_semantic::db::SemanticGroup;
use cairo_lang_semantic::items::attribute::SemanticQueryAttrs;
use cairo_lang_semantic::plugin::PluginSuite;
use cairo_lang_syntax::attribute::structured::{
    Attribute, AttributeArgVariant, AttributeStructurize,
};
use cairo_lang_syntax::node::ast::{Expr, ImplItem, MaybeImplBody, MaybeTraitBody, PathSegment};
use cairo_lang_syntax::node::helpers::QueryAttrs;
use cairo_lang_syntax::node::ids::SyntaxStablePtrId;
use cairo_lang_syntax::node::{Terminal, TypedStablePtr, TypedSyntaxNode, ast};
use itertools::Itertools;
use salsa::Database;
use scarb_stable_hash::{StableHasher, short_hash};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock, RwLock};
use std::vec::IntoIter;
use tracing::{debug, trace_span};

const DERIVE_ATTR: &str = "derive";

/// A Cairo compiler plugin controlling the procedural macro execution.
///
/// This plugin decides which macro plugins (if any) should be applied to the processed AST item.
/// It then redirects the item to the appropriate macro plugin for code expansion.
#[derive(Debug)]
pub struct ProcMacroHostPlugin {
    instances: Vec<Arc<ProcMacroInstance>>,
    full_path_markers: RwLock<HashMap<PackageId, Vec<String>>>,
}

impl DeclaredProcMacroInstances for ProcMacroHostPlugin {
    fn instances(&self) -> &[Arc<ProcMacroInstance>] {
        &self.instances
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ProcMacroId {
    pub package_id: PackageId,
    pub expansion: Expansion,
}

impl ProcMacroId {
    pub fn new(package_id: PackageId, expansion: Expansion) -> Self {
        Self {
            package_id,
            expansion,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProcMacroAuxData {
    value: Vec<u8>,
    macro_id: ProcMacroId,
}

impl ProcMacroAuxData {
    pub fn new(value: Vec<u8>, macro_id: ProcMacroId) -> Self {
        Self { value, macro_id }
    }
}

impl From<ProcMacroAuxData> for AuxData {
    fn from(data: ProcMacroAuxData) -> Self {
        Self::new(data.value)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmittedAuxData(Vec<ProcMacroAuxData>);

#[typetag::serde]
impl GeneratedFileAuxData for EmittedAuxData {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq(&self, other: &dyn GeneratedFileAuxData) -> bool {
        self.0 == other.as_any().downcast_ref::<Self>().unwrap().0
    }

    fn hash_value(&self) -> u64 {
        let mut hasher = StableHasher::new();
        for aux_data in &self.0 {
            aux_data.value.hash(&mut hasher);
            aux_data.macro_id.hash(&mut hasher);
        }
        hasher.finish()
    }
}

impl EmittedAuxData {
    pub fn new(aux_data: ProcMacroAuxData) -> Self {
        Self(vec![aux_data])
    }

    pub fn push(&mut self, aux_data: ProcMacroAuxData) {
        self.0.push(aux_data);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl IntoIterator for EmittedAuxData {
    type Item = ProcMacroAuxData;
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> IntoIter<ProcMacroAuxData> {
        self.0.into_iter()
    }
}

impl ProcMacroHostPlugin {
    pub fn try_new(macros: Vec<Arc<ProcMacroInstance>>) -> Result<Self> {
        // Validate expansions.
        let mut expansions = macros
            .iter()
            .flat_map(|m| {
                m.get_expansions()
                    .iter()
                    .map(|e| ProcMacroId::new(m.package_id(), e.clone()))
                    .collect_vec()
            })
            .collect::<Vec<_>>();
        expansions.sort_unstable_by_key(|e| (e.expansion.cairo_name.clone(), e.package_id));
        ensure!(
            expansions
                .windows(2)
                .all(|w| w[0].expansion.cairo_name != w[1].expansion.cairo_name),
            "duplicate expansions defined for procedural macros: {duplicates}",
            duplicates = expansions
                .windows(2)
                .filter(|w| w[0].expansion.cairo_name == w[1].expansion.cairo_name)
                .map(|w| format!(
                    "{} ({} and {})",
                    w[0].expansion.cairo_name.as_str(),
                    w[0].package_id,
                    w[1].package_id
                ))
                .collect::<Vec<_>>()
                .join(", ")
        );
        Ok(Self {
            instances: macros,
            full_path_markers: RwLock::new(Default::default()),
        })
    }

    fn uses_proc_macros<'db>(
        &self,
        db: &'db dyn Database,
        item_ast: &ast::ModuleItem<'db>,
    ) -> bool {
        // Check on inner attributes too.
        let inner_attrs: HashSet<_> = match item_ast {
            ast::ModuleItem::Impl(imp) => {
                if let MaybeImplBody::Some(body) = imp.body(db) {
                    body.items(db)
                        .elements(db)
                        .flat_map(|item| item.attributes_elements(db).collect_vec())
                        .map(|attr| {
                            attr.attr(db)
                                .as_syntax_node()
                                .get_text_without_trivia(db)
                                .to_string(db)
                        })
                        .collect()
                } else {
                    Default::default()
                }
            }
            ast::ModuleItem::Trait(trt) => {
                if let MaybeTraitBody::Some(body) = trt.body(db) {
                    body.items(db)
                        .elements(db)
                        .flat_map(|item| item.attributes_elements(db).collect_vec())
                        .map(|attr| {
                            attr.attr(db)
                                .as_syntax_node()
                                .get_text_without_trivia(db)
                                .to_string(db)
                        })
                        .collect()
                } else {
                    Default::default()
                }
            }
            _ => Default::default(),
        };

        if !DeclaredProcMacroInstances::declared_attributes(self).into_iter().any(|declared_attr|
            item_ast.has_attr(db, &declared_attr) || inner_attrs.contains(&declared_attr)
        )
            // Plugins can implement own derives.
            && !item_ast.has_attr(db, "derive")
            // Plugins does not declare module inline macros they support.
            && !matches!(item_ast, ast::ModuleItem::InlineMacro(_))
        {
            return false;
        };
        true
    }

    fn expand_inner_attr<'db>(
        &self,
        db: &'db dyn Database,
        item_ast: ast::ModuleItem<'db>,
    ) -> InnerAttrExpansionResult<'db> {
        let mut context = InnerAttrExpansionContext::new(self);
        let mut item_builder = PatchBuilder::new(db, &item_ast);
        let mut used_attr_names: HashSet<SmolStr> = Default::default();
        let mut all_none = true;

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
                        for item in item_list.elements(db) {
                            let ast::TraitItem::Function(func) = item else {
                                item_builder.add_node(item.as_syntax_node());
                                continue;
                            };

                            let mut func_builder = PatchBuilder::new(db, &func);
                            let attrs = func.attributes(db).elements(db).collect();
                            let found = self.parse_attrs(db, &mut func_builder, attrs, &func);
                            if let Some(name) = found.as_name() {
                                used_attr_names.insert(name);
                            }
                            func_builder.add_node(func.declaration(db).as_syntax_node());
                            func_builder.add_node(func.body(db).as_syntax_node());
                            let token_stream = TokenStream::new(func_builder.build().0);

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
                                db,
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

                            let mut func_builder = PatchBuilder::new(db, &func);
                            let attrs = func.attributes(db).elements(db).collect();
                            let found = self.parse_attrs(db, &mut func_builder, attrs, &func);
                            if let Some(name) = found.as_name() {
                                used_attr_names.insert(name);
                            }
                            func_builder.add_node(func.visibility(db).as_syntax_node());
                            func_builder.add_node(func.declaration(db).as_syntax_node());
                            func_builder.add_node(func.body(db).as_syntax_node());
                            let token_stream = TokenStream::new(func_builder.build().0);
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
                                db,
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

    fn do_expand_inner_attr<'db>(
        &self,
        db: &'db dyn Database,
        context: &mut InnerAttrExpansionContext<'_, 'db>,
        item_builder: &mut PatchBuilder<'db>,
        found: AttrExpansionFound<'db>,
        func: &impl TypedSyntaxNode<'db>,
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

        let result = self
            .instance(input.package_id)
            .try_v1()
            .expect("procedural macro using v2 api used in a context expecting v1 api")
            .generate_code(
                input.expansion.expansion_name.clone(),
                args.clone(),
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
    fn parse_attribute<'db>(
        &self,
        db: &'db dyn Database,
        item_ast: ast::ModuleItem<'db>,
    ) -> (AttrExpansionFound<'db>, TokenStream) {
        let mut item_builder = PatchBuilder::new(db, &item_ast);
        let input = match item_ast.clone() {
            ast::ModuleItem::Trait(trait_ast) => {
                let attrs = trait_ast.attributes(db).elements(db).collect();
                let expansion = self.parse_attrs(db, &mut item_builder, attrs, &item_ast);
                item_builder.add_node(trait_ast.visibility(db).as_syntax_node());
                item_builder.add_node(trait_ast.trait_kw(db).as_syntax_node());
                item_builder.add_node(trait_ast.name(db).as_syntax_node());
                item_builder.add_node(trait_ast.generic_params(db).as_syntax_node());
                item_builder.add_node(trait_ast.body(db).as_syntax_node());
                expansion
            }
            ast::ModuleItem::Impl(impl_ast) => {
                let attrs = impl_ast.attributes(db).elements(db).collect();
                let expansion = self.parse_attrs(db, &mut item_builder, attrs, &item_ast);
                item_builder.add_node(impl_ast.visibility(db).as_syntax_node());
                item_builder.add_node(impl_ast.impl_kw(db).as_syntax_node());
                item_builder.add_node(impl_ast.name(db).as_syntax_node());
                item_builder.add_node(impl_ast.generic_params(db).as_syntax_node());
                item_builder.add_node(impl_ast.of_kw(db).as_syntax_node());
                item_builder.add_node(impl_ast.trait_path(db).as_syntax_node());
                item_builder.add_node(impl_ast.body(db).as_syntax_node());
                expansion
            }
            ast::ModuleItem::Module(module_ast) => {
                let attrs = module_ast.attributes(db).elements(db).collect();
                let expansion = self.parse_attrs(db, &mut item_builder, attrs, &item_ast);
                item_builder.add_node(module_ast.visibility(db).as_syntax_node());
                item_builder.add_node(module_ast.module_kw(db).as_syntax_node());
                item_builder.add_node(module_ast.name(db).as_syntax_node());
                item_builder.add_node(module_ast.body(db).as_syntax_node());
                expansion
            }
            ast::ModuleItem::FreeFunction(free_func_ast) => {
                let attrs = free_func_ast.attributes(db).elements(db).collect();
                let expansion = self.parse_attrs(db, &mut item_builder, attrs, &item_ast);
                item_builder.add_node(free_func_ast.visibility(db).as_syntax_node());
                item_builder.add_node(free_func_ast.declaration(db).as_syntax_node());
                item_builder.add_node(free_func_ast.body(db).as_syntax_node());
                expansion
            }
            ast::ModuleItem::ExternFunction(extern_func_ast) => {
                let attrs = extern_func_ast.attributes(db).elements(db).collect();
                let expansion = self.parse_attrs(db, &mut item_builder, attrs, &item_ast);
                item_builder.add_node(extern_func_ast.visibility(db).as_syntax_node());
                item_builder.add_node(extern_func_ast.extern_kw(db).as_syntax_node());
                item_builder.add_node(extern_func_ast.declaration(db).as_syntax_node());
                item_builder.add_node(extern_func_ast.semicolon(db).as_syntax_node());
                expansion
            }
            ast::ModuleItem::ExternType(extern_type_ast) => {
                let attrs = extern_type_ast.attributes(db).elements(db).collect();
                let expansion = self.parse_attrs(db, &mut item_builder, attrs, &item_ast);
                item_builder.add_node(extern_type_ast.visibility(db).as_syntax_node());
                item_builder.add_node(extern_type_ast.extern_kw(db).as_syntax_node());
                item_builder.add_node(extern_type_ast.type_kw(db).as_syntax_node());
                item_builder.add_node(extern_type_ast.name(db).as_syntax_node());
                item_builder.add_node(extern_type_ast.generic_params(db).as_syntax_node());
                item_builder.add_node(extern_type_ast.semicolon(db).as_syntax_node());
                expansion
            }
            ast::ModuleItem::Struct(struct_ast) => {
                let attrs = struct_ast.attributes(db).elements(db).collect();
                let expansion = self.parse_attrs(db, &mut item_builder, attrs, &item_ast);
                item_builder.add_node(struct_ast.visibility(db).as_syntax_node());
                item_builder.add_node(struct_ast.struct_kw(db).as_syntax_node());
                item_builder.add_node(struct_ast.name(db).as_syntax_node());
                item_builder.add_node(struct_ast.generic_params(db).as_syntax_node());
                item_builder.add_node(struct_ast.lbrace(db).as_syntax_node());
                item_builder.add_node(struct_ast.members(db).as_syntax_node());
                item_builder.add_node(struct_ast.rbrace(db).as_syntax_node());
                expansion
            }
            ast::ModuleItem::Enum(enum_ast) => {
                let attrs = enum_ast.attributes(db).elements(db).collect();
                let expansion = self.parse_attrs(db, &mut item_builder, attrs, &item_ast);
                item_builder.add_node(enum_ast.visibility(db).as_syntax_node());
                item_builder.add_node(enum_ast.enum_kw(db).as_syntax_node());
                item_builder.add_node(enum_ast.name(db).as_syntax_node());
                item_builder.add_node(enum_ast.generic_params(db).as_syntax_node());
                item_builder.add_node(enum_ast.lbrace(db).as_syntax_node());
                item_builder.add_node(enum_ast.variants(db).as_syntax_node());
                item_builder.add_node(enum_ast.rbrace(db).as_syntax_node());
                expansion
            }
            _ => AttrExpansionFound::None,
        };
        let token_stream = TokenStream::new(item_builder.build().0);
        (input, token_stream)
    }

    fn parse_attrs<'db>(
        &self,
        db: &'db dyn Database,
        builder: &mut PatchBuilder<'db>,
        attrs: Vec<ast::Attribute<'db>>,
        origin: &impl TypedSyntaxNode<'db>,
    ) -> AttrExpansionFound<'db> {
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
                let found = self.find_expansion(&ExpansionQuery::with_cairo_name(
                    structured_attr.id.to_string(db),
                    ExpansionKind::Attr,
                ));
                if let Some(found) = found {
                    if expansion.is_none() {
                        let mut args_builder = PatchBuilder::new(db, origin);
                        args_builder.add_node(attr.arguments(db).as_syntax_node());
                        let args = TokenStream::new(args_builder.build().0);
                        expansion = Some((found, args, attr.stable_ptr(db).untyped()));
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

    /// Handle `#[derive(...)]` attribute.
    ///
    /// Returns a list of expansions that this plugin should apply.
    fn parse_derive<'db>(
        &self,
        db: &'db dyn Database,
        item_ast: ast::ModuleItem<'db>,
    ) -> Vec<ProcMacroId> {
        let attrs = match item_ast {
            ast::ModuleItem::Struct(struct_ast) => {
                Some(struct_ast.query_attr(db, DERIVE_ATTR).collect_vec())
            }
            ast::ModuleItem::Enum(enum_ast) => {
                Some(enum_ast.query_attr(db, DERIVE_ATTR).collect_vec())
            }
            _ => None,
        };

        attrs
            .unwrap_or_default()
            .iter()
            .map(|attr| attr.clone().structurize(db))
            .flat_map(|attr| attr.args.into_iter())
            .filter_map(|attr| {
                let AttributeArgVariant::Unnamed(value) = attr.clone().variant else {
                    return None;
                };
                let Expr::Path(path) = value else {
                    return None;
                };
                let path = path.segments(db);
                let path = path.elements(db);
                let path = path.last()?;
                let PathSegment::Simple(segment) = path else {
                    return None;
                };
                let ident = segment.ident(db);
                let value = ident.text(db).to_string(db);
                self.find_expansion(&ExpansionQuery::with_cairo_name(
                    value,
                    ExpansionKind::Derive,
                ))
            })
            .collect_vec()
    }

    fn expand_derives<'db>(
        &self,
        db: &'db dyn Database,
        item_ast: ast::ModuleItem<'db>,
        derives: Vec<ProcMacroId>,
        stream_metadata: TokenStreamMetadata,
    ) -> Option<PluginResult<'db>> {
        let stable_ptr = item_ast.clone().stable_ptr(db).untyped();
        let token_stream =
            TokenStream::from_syntax_node(db, &item_ast).with_metadata(stream_metadata.clone());

        let mut aux_data = EmittedAuxData::default();
        let mut all_diagnostics: Vec<Diagnostic> = Vec::new();

        let any_derives = !derives.is_empty();

        let mut derived_code = PatchBuilder::new(db, &item_ast);
        for derive in derives.iter() {
            let result = self
                .instance(derive.package_id)
                .try_v1()
                .expect("procedural macro using v2 api used in a context expecting v1 api")
                .generate_code(
                    derive.expansion.expansion_name.clone(),
                    TokenStream::empty(),
                    token_stream.clone(),
                );

            // Register diagnostics.
            all_diagnostics.extend(result.diagnostics);

            // Register aux data.
            if let Some(new_aux_data) = result.aux_data {
                aux_data.push(ProcMacroAuxData::new(
                    new_aux_data.into(),
                    ProcMacroId::new(derive.package_id, derive.expansion.clone()),
                ));
            }

            if result.token_stream.is_empty() {
                // No code has been generated.
                // We do not need to do anything.
                continue;
            }

            derived_code.add_str(result.token_stream.to_string().as_str());
        }

        if any_derives {
            let derived_code = derived_code.build().0;
            return Some(PluginResult {
                code: if derived_code.is_empty() {
                    None
                } else {
                    let msg = if derives.len() == 1 {
                        "the derive macro"
                    } else {
                        "one of the derive macros"
                    };
                    let derive_names = derives
                        .iter()
                        .map(|derive| derive.expansion.cairo_name.to_string())
                        .join("`, `");
                    let note = format!("this error originates in {msg}: `{derive_names}`");
                    Some(PluginGeneratedFile {
                        name: "proc_macro_derive".into(),
                        code_mappings: Vec::new(),
                        content: derived_code,
                        aux_data: if aux_data.is_empty() {
                            None
                        } else {
                            Some(DynGeneratedFileAuxData::new(aux_data))
                        },
                        diagnostics_note: Some(note),
                        is_unhygienic: false,
                    })
                },
                diagnostics: into_cairo_diagnostics(all_diagnostics, stable_ptr),
                // Note that we don't remove the original item here, unlike for attributes.
                // We do not add the original code to the generated file either.
                remove_original_item: false,
            });
        }

        None
    }

    fn expand_attribute<'db>(
        &self,
        input: ProcMacroId,
        last: bool,
        args: TokenStream,
        token_stream: TokenStream,
        stable_ptr: SyntaxStablePtrId<'db>,
    ) -> PluginResult<'db> {
        let result = self
            .instance(input.package_id)
            .try_v1()
            .expect("procedural macro using v2 api used in a context expecting v1 api")
            .generate_code(
                input.expansion.expansion_name.clone(),
                args.clone(),
                token_stream.clone(),
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
        if last
            && result.aux_data.is_none()
            && token_stream.to_string() == result.token_stream.to_string()
        {
            return PluginResult {
                code: None,
                remove_original_item: false,
                diagnostics: into_cairo_diagnostics(result.diagnostics, stable_ptr),
            };
        }

        let file_name = format!("proc_{}", input.expansion.cairo_name);
        let content = result.token_stream.to_string();
        PluginResult {
            code: Some(PluginGeneratedFile {
                name: file_name,
                code_mappings: Vec::new(),
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
                is_unhygienic: false,
            }),
            diagnostics: into_cairo_diagnostics(result.diagnostics, stable_ptr),
            remove_original_item: true,
        }
    }

    fn find_expansion(&self, query: &ExpansionQuery) -> Option<ProcMacroId> {
        let instance = self.find_instance_with_expansion(query)?;
        let expansion = instance.find_expansion(query)?;
        Some(ProcMacroId::new(instance.package_id(), expansion.clone()))
    }

    pub fn build_plugin_suite(macro_host: Arc<Self>) -> PluginSuite {
        let mut suite = PluginSuite::default();
        // Register inline macro plugins.
        for proc_macro in &macro_host.instances {
            let expansions = proc_macro
                .get_expansions()
                .iter()
                .filter(|exp| matches!(exp.kind, ExpansionKind::Inline));
            for expansion in expansions {
                let plugin = Arc::new(ProcMacroInlinePlugin::new(
                    proc_macro.clone(),
                    expansion.clone(),
                ));
                suite.add_inline_macro_plugin_ex(expansion.cairo_name.as_str(), plugin);
            }
        }
        // Register procedural macro host plugin.
        suite.add_plugin_ex(macro_host);
        suite
    }

    #[tracing::instrument(level = "trace", skip_all)]
    pub fn post_process(&self, db: &dyn SemanticGroup) -> Result<()> {
        let aux_data = self.collect_aux_data(db);
        // Only collect full path markers if any have been registered by macros.
        let any_markers = !self
            .full_path_markers
            .read()
            .unwrap()
            .values()
            .all(|v| v.is_empty());
        let markers = if any_markers {
            self.collect_full_path_markers(db)
        } else {
            Default::default()
        };
        for instance in self.instances.iter() {
            let _ = trace_span!(
                "post_process_callback",
                instance = %instance.package_id()
            )
            .entered();
            let instance_markers = if any_markers {
                {
                    self.full_path_markers
                        .read()
                        .unwrap()
                        .get(&instance.package_id())
                        .cloned()
                        .unwrap_or_default()
                }
            } else {
                Default::default()
            };
            let markers_for_instance = markers
                .iter()
                .filter(|(key, _)| instance_markers.contains(key))
                .map(|(key, full_path)| FullPathMarker {
                    key: key.clone(),
                    full_path: full_path.clone(),
                })
                .collect_vec();
            let data = aux_data
                .get(&instance.package_id())
                .cloned()
                .unwrap_or_default();
            debug!("calling post processing callback with: {data:?}");
            instance
                .try_v1()
                .expect("procedural macro using v2 api used in a context expecting v1 api")
                .post_process_callback(data.clone(), markers_for_instance);
        }
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip_all)]
    fn collect_full_path_markers(&self, db: &dyn SemanticGroup) -> HashMap<String, String> {
        let mut markers: HashMap<String, String> = HashMap::new();
        for crate_id in db.crates() {
            let modules = db.crate_modules(*crate_id);
            for module_id in modules.iter() {
                let Ok(module_data) = module_id.module_data(db) else {
                    continue;
                };
                let module_items = module_data.items(db);
                for item_id in module_items.iter() {
                    let attr: Option<Vec<_>> = match item_id {
                        ModuleItemId::Struct(id) => id
                            .query_attr(db, FULL_PATH_MARKER_KEY)
                            .map(|x| x.into_iter().collect())
                            .to_option(),
                        ModuleItemId::Enum(id) => id
                            .query_attr(db, FULL_PATH_MARKER_KEY)
                            .map(|x| x.into_iter().collect())
                            .to_option(),
                        ModuleItemId::FreeFunction(id) => id
                            .query_attr(db, FULL_PATH_MARKER_KEY)
                            .map(|x| x.into_iter().collect())
                            .to_option(),
                        _ => None,
                    };
                    let keys = attr
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|attr| Self::extract_key(db, attr))
                        .collect_vec();
                    let full_path = item_id.full_path(db);
                    for key in keys {
                        markers.insert(key, full_path.clone());
                    }
                }
            }
        }
        markers
    }

    fn extract_key<'db>(db: &'db dyn SemanticGroup, attr: &'db Attribute<'db>) -> Option<String> {
        if attr.id.to_string(db) != FULL_PATH_MARKER_KEY {
            return None;
        }

        for arg in attr.args.clone() {
            if let AttributeArgVariant::Unnamed(Expr::String(s)) = arg.variant {
                return s.string_value(db);
            }
        }

        None
    }

    #[tracing::instrument(level = "trace", skip_all)]
    fn collect_aux_data(
        &self,
        db: &dyn SemanticGroup,
    ) -> HashMap<PackageId, Vec<ProcMacroAuxData>> {
        let mut data = Vec::new();
        for crate_id in db.crates() {
            let crate_modules = db.crate_modules(*crate_id);
            for module in crate_modules.iter() {
                if let Ok(module_data) = module.module_data(db) {
                    for (_file_id, aux_data_option) in
                        module_data.generated_file_aux_data(db).iter()
                    {
                        let aux_data = aux_data_option
                            .as_ref()
                            .and_then(|ad| ad.as_any().downcast_ref::<EmittedAuxData>());
                        if let Some(aux_data) = aux_data {
                            data.extend(aux_data.clone().into_iter());
                        }
                    }
                }
            }
        }
        data.into_iter()
            .into_group_map_by(|d| d.macro_id.package_id)
    }

    pub fn instance(&self, package_id: PackageId) -> &ProcMacroInstance {
        self.instances
            .iter()
            .find(|m| m.package_id() == package_id)
            .expect("procedural macro must be registered in proc macro host")
    }

    fn register_full_path_markers(&self, package_id: PackageId, markers: Vec<String>) {
        self.full_path_markers
            .write()
            .unwrap()
            .entry(package_id)
            .and_modify(|markers| markers.extend(markers.clone()))
            .or_insert(markers);
    }

    fn calculate_metadata<'db>(
        db: &'db dyn Database,
        item_ast: ast::ModuleItem<'db>,
    ) -> TokenStreamMetadata {
        let stable_ptr = item_ast.clone().stable_ptr(db).untyped();
        let file_path = stable_ptr.file_id(db).full_path(db);
        let file_id = short_hash(file_path.clone());
        TokenStreamMetadata::new(file_path, file_id)
    }
}

struct InnerAttrExpansionContext<'a, 'db> {
    host: &'a ProcMacroHostPlugin,
    // Metadata returned for expansions.
    diagnostics: Vec<PluginDiagnostic<'db>>,
    aux_data: EmittedAuxData,
    any_changed: bool,
}

impl<'a, 'db> InnerAttrExpansionContext<'a, 'db> {
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
        stable_ptr: SyntaxStablePtrId<'db>,
    ) -> String {
        let expanded = result.token_stream.to_string();
        let changed = expanded.as_str() != original;

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

        expanded
    }
    pub fn into_result(
        self,
        _db: &'db dyn Database,
        expanded: String,
        code_mappings: Vec<CodeMapping>,
        attr_names: Vec<SmolStr>,
    ) -> PluginResult<'db> {
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
                is_unhygienic: false,
            }),
            diagnostics: self.diagnostics,
            remove_original_item: true,
        }
    }
}

enum InnerAttrExpansionResult<'db> {
    None,
    Some(PluginResult<'db>),
}

impl MacroPlugin for ProcMacroHostPlugin {
    #[tracing::instrument(level = "trace", skip_all)]
    fn generate_code<'db>(
        &self,
        db: &'db dyn Database,
        item_ast: ast::ModuleItem<'db>,
        _metadata: &MacroPluginMetadata<'_>,
    ) -> PluginResult<'db> {
        // We first check if the ast item uses any proc macros. If not, we exit early.
        // This is strictly a performance optimization, as gathering expansion metadata can be costly.
        if !self.uses_proc_macros(db, &item_ast) {
            return Default::default();
        };

        let stream_metadata = Self::calculate_metadata(db, item_ast.clone());

        // Handle inner functions.
        if let InnerAttrExpansionResult::Some(result) = self.expand_inner_attr(db, item_ast.clone())
        {
            return result;
        }

        // Expand first attribute.
        // Note that we only expand the first attribute, as we assume that the rest of the attributes
        // will be handled by a subsequent call to this function.
        let (input, body) = self.parse_attribute(db, item_ast.clone());

        // All derives to be applied.
        let derives = self.parse_derive(db, item_ast.clone());

        if let Some(result) = match input {
            AttrExpansionFound::Last {
                expansion,
                args,
                stable_ptr,
            } => Some((expansion, args, stable_ptr, true)),
            AttrExpansionFound::Some {
                expansion,
                args,
                stable_ptr,
            } => Some((expansion, args, stable_ptr, false)),
            AttrExpansionFound::None => None,
        }
        .map(|(expansion, args, stable_ptr, last)| {
            let token_stream = body.with_metadata(stream_metadata.clone());
            self.expand_attribute(
                expansion,
                // We also want to mark that this is the last attribute if there are no derives to be applied.
                last && derives.is_empty(),
                args,
                token_stream,
                stable_ptr,
            )
        }) {
            return result;
        }

        // Expand all derives.
        // Note that all proc macro attributes should be already expanded at this point.
        if let Some(result) =
            self.expand_derives(db, item_ast.clone(), derives, stream_metadata.clone())
        {
            return result;
        }

        // No expansions can be applied.
        PluginResult {
            code: None,
            diagnostics: Vec::new(),
            remove_original_item: false,
        }
    }

    fn declared_attributes<'db>(&self, db: &'db dyn Database) -> Vec<SmolStrId<'db>> {
        DeclaredProcMacroInstances::declared_attributes(self)
            .into_iter()
            .map(|s| SmolStrId::from(db, s))
            .collect()
    }

    fn declared_derives<'db>(&self, db: &'db dyn Database) -> Vec<SmolStrId<'db>> {
        DeclaredProcMacroInstances::declared_derives(self)
            .into_iter()
            .map(|s| SmolStrId::from(db, s))
            .collect()
    }

    fn executable_attributes<'db>(&self, db: &'db dyn Database) -> Vec<SmolStrId<'db>> {
        DeclaredProcMacroInstances::executable_attributes(self)
            .into_iter()
            .map(|s| SmolStrId::from(db, s))
            .collect()
    }
}

enum AttrExpansionFound<'db> {
    Some {
        expansion: ProcMacroId,
        args: TokenStream,
        stable_ptr: SyntaxStablePtrId<'db>,
    },
    None,
    Last {
        expansion: ProcMacroId,
        args: TokenStream,
        stable_ptr: SyntaxStablePtrId<'db>,
    },
}

impl<'db> AttrExpansionFound<'db> {
    pub fn as_name(&self) -> Option<SmolStr> {
        match self {
            AttrExpansionFound::Some { expansion, .. }
            | AttrExpansionFound::Last { expansion, .. } => {
                Some(expansion.expansion.cairo_name.clone())
            }
            AttrExpansionFound::None => None,
        }
    }
}

/// A Cairo compiler inline macro plugin controlling the inline procedural macro execution.
///
/// This plugin represents a single expansion capable of handling inline procedural macros.
/// The plugin triggers code expansion in a corresponding procedural macro instance.
#[derive(Debug)]
pub struct ProcMacroInlinePlugin {
    instance: Arc<ProcMacroInstance>,
    expansion: Expansion,
    doc: OnceLock<Option<String>>,
}

impl ProcMacroInlinePlugin {
    pub fn new(instance: Arc<ProcMacroInstance>, expansion: Expansion) -> Self {
        assert!(instance.get_expansions().contains(&expansion));
        Self {
            instance,
            expansion,
            doc: Default::default(),
        }
    }

    pub fn name(&self) -> &str {
        self.expansion.cairo_name.as_str()
    }

    fn instance(&self) -> &ProcMacroInstance {
        &self.instance
    }
}

impl InlineMacroExprPlugin for ProcMacroInlinePlugin {
    #[tracing::instrument(level = "trace", skip_all)]
    fn generate_code<'db>(
        &self,
        db: &'db dyn Database,
        syntax: &ast::ExprInlineMacro<'db>,
        _metadata: &MacroPluginMetadata<'_>,
    ) -> InlinePluginResult<'db> {
        let stable_ptr = syntax.clone().stable_ptr(db).untyped();
        let arguments = syntax.arguments(db);
        let token_stream = TokenStream::from_syntax_node(db, &arguments);
        let result = self
            .instance()
            .try_v1()
            .expect("procedural macro using v2 api used in a context expecting v1 api")
            .generate_code(
                self.expansion.expansion_name.clone(),
                TokenStream::empty(),
                token_stream,
            );
        // Handle diagnostics.
        let diagnostics = into_cairo_diagnostics(result.diagnostics, stable_ptr);
        let token_stream = result.token_stream.clone();
        if token_stream.is_empty() {
            // Remove original code
            InlinePluginResult {
                code: None,
                diagnostics,
            }
        } else {
            // Replace
            let aux_data = result.aux_data.map(|aux_data| {
                let aux_data = ProcMacroAuxData::new(
                    aux_data.into(),
                    ProcMacroId::new(self.instance.package_id(), self.expansion.clone()),
                );
                let mut emitted = EmittedAuxData::default();
                emitted.push(aux_data);
                DynGeneratedFileAuxData::new(emitted)
            });
            let content = token_stream.to_string();
            InlinePluginResult {
                code: Some(PluginGeneratedFile {
                    name: "inline_proc_macro".into(),
                    code_mappings: Vec::new(),
                    content,
                    aux_data,
                    diagnostics_note: Some(format!(
                        "this error originates in the inline macro: `{}`",
                        self.expansion.cairo_name
                    )),
                    is_unhygienic: false,
                }),
                diagnostics,
            }
        }
    }

    fn documentation(&self) -> Option<String> {
        self.doc
            .get_or_init(|| self.instance().doc(self.expansion.cairo_name.clone()))
            .clone()
    }
}

fn into_cairo_diagnostics<'db>(
    diagnostics: Vec<Diagnostic>,
    stable_ptr: SyntaxStablePtrId<'db>,
) -> Vec<PluginDiagnostic<'db>> {
    diagnostics
        .into_iter()
        .map(|diag| PluginDiagnostic {
            stable_ptr,
            message: diag.message,
            severity: match diag.severity {
                Severity::Error => cairo_lang_diagnostics::Severity::Error,
                Severity::Warning => cairo_lang_diagnostics::Severity::Warning,
            },
            inner_span: None,
        })
        .collect_vec()
}
