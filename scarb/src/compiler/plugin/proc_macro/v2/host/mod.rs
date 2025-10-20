mod attribute;
mod aux_data;
mod conversion;
pub mod derive;
mod inline;
mod post;

use attribute::*;
pub use aux_data::ProcMacroAuxData;
use inline::*;
use serde::{Deserialize, Serialize};

use crate::compiler::plugin::proc_macro::expansion::{Expansion, ExpansionKind};
use crate::compiler::plugin::proc_macro::{
    DeclaredProcMacroInstances, ExpansionQuery, ProcMacroInstance,
};
use crate::core::{PackageId, edition_variant};
use anyhow::{Result, ensure};
use cairo_lang_defs::plugin::{MacroPlugin, MacroPluginMetadata, PluginResult};
use cairo_lang_filesystem::db::Edition;
use cairo_lang_filesystem::ids::{CodeMapping, CodeOrigin, SmolStrId};
use cairo_lang_filesystem::span::{TextOffset, TextSpan, TextWidth};
use cairo_lang_macro::{
    AllocationContext, TextSpan as MacroTextSpan, TokenStream, TokenStreamMetadata, TokenTree,
};
use cairo_lang_semantic::plugin::PluginSuite;
use cairo_lang_syntax::node::ast::{MaybeImplBody, MaybeTraitBody};
use cairo_lang_syntax::node::helpers::QueryAttrs;
use cairo_lang_syntax::node::{TypedStablePtr, TypedSyntaxNode, ast};
use itertools::Itertools;
use salsa::Database;
use scarb_stable_hash::short_hash;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::sync::{Arc, RwLock};

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
                        .map(|attr| attr.attr(db).as_syntax_node().get_text_without_trivia(db))
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
                        .map(|attr| attr.attr(db).as_syntax_node().get_text_without_trivia(db))
                        .collect()
                } else {
                    Default::default()
                }
            }
            _ => Default::default(),
        };

        if !DeclaredProcMacroInstances::declared_attributes(self).into_iter().any(|declared_attr|
            item_ast.has_attr(db, &declared_attr) || inner_attrs.contains(&SmolStrId::from(db, declared_attr))
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

    pub(crate) fn find_expansion(&self, query: &ExpansionQuery) -> Option<ProcMacroId> {
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

    pub fn instance(&self, package_id: PackageId) -> &ProcMacroInstance {
        self.instances
            .iter()
            .find(|m| m.package_id() == package_id)
            .expect("procedural macro must be registered in proc macro host")
    }

    fn calculate_metadata<'db>(
        db: &'db dyn Database,
        item_ast: ast::ModuleItem<'db>,
        edition: Edition,
    ) -> TokenStreamMetadata {
        let stable_ptr = item_ast.clone().stable_ptr(db).untyped();
        let file_path = stable_ptr.file_id(db).full_path(db);
        let file_id = short_hash(file_path.clone());
        let edition = edition_variant(edition);
        TokenStreamMetadata::new(file_path, file_id, edition)
    }
}

impl MacroPlugin for ProcMacroHostPlugin {
    #[tracing::instrument(level = "trace", skip_all)]
    fn generate_code<'db>(
        &self,
        db: &'db dyn Database,
        item_ast: ast::ModuleItem<'db>,
        metadata: &MacroPluginMetadata<'_>,
    ) -> PluginResult<'db> {
        // We first check if the ast item uses any proc macros. If not, we exit early.
        // This is strictly a performance optimization, as gathering expansion metadata can be costly.
        if !self.uses_proc_macros(db, &item_ast) {
            return Default::default();
        };

        let stream_metadata = Self::calculate_metadata(db, item_ast.clone(), metadata.edition);

        // Expand module-level inline macro.
        if let ast::ModuleItem::InlineMacro(inline_macro) = &item_ast
            && let Some(result) =
                expand_module_level_inline_macro(self, db, inline_macro, &stream_metadata)
            {
                return result;
            }

        // Handle inner functions.
        if let InnerAttrExpansionResult::Some(result) = self.expand_inner_attr(db, item_ast.clone())
        {
            return result.into();
        }

        // Expand the first attribute.
        // Note that we only expand the first attribute, as we assume that the rest of the attributes
        // will be handled by a subsequent call to this function.
        let ctx = AllocationContext::default();
        let (input, body) = self.parse_attribute(db, item_ast.clone(), &ctx);

        if let Some(result) = match input {
            AttrExpansionFound::Last(input) => Some((input, true)),
            AttrExpansionFound::Some(input) => Some((input, false)),
            AttrExpansionFound::None => None,
        }
        .map(|(input, last)| {
            let token_stream = body.with_metadata(stream_metadata.clone());
            self.expand_attribute(db, last, input.args.clone(), token_stream, input)
        }) {
            return result.into();
        }

        // Expand all derives.
        // Note that all proc macro attributes should be already expanded at this point.
        if let Some(result) = self.expand_derives(db, item_ast.clone(), stream_metadata.clone()) {
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

pub fn generate_code_mappings(
    token_stream: &TokenStream,
    call_site: MacroTextSpan,
) -> Vec<CodeMapping> {
    let mappings: Vec<CodeMapping> = token_stream
        .tokens
        .iter()
        .scan(TextOffset::default(), |current_pos, token| {
            let TokenTree::Ident(token) = token;
            let token_width = TextWidth::from_str(token.content.as_ref());

            let mapping = CodeMapping {
                span: TextSpan {
                    start: *current_pos,
                    end: current_pos.add_width(token_width),
                },
                origin: CodeOrigin::Span(TextSpan {
                    start: TextOffset::default()
                        .add_width(TextWidth::new_for_testing(token.span.start)),
                    end: TextOffset::default()
                        .add_width(TextWidth::new_for_testing(token.span.end)),
                }),
            };

            *current_pos = current_pos.add_width(token_width);
            Some(mapping)
        })
        .collect();
    let mut mappings = mappings
        .clone()
        .into_iter()
        // Emit additional mappings at the start of a span for zero-width diagnostics.
        .flat_map(|mapping| match &mapping.origin {
            CodeOrigin::Span(origin) => {
                if origin.start.as_u32() == call_site.start && origin.end.as_u32() == call_site.end
                {
                    // Call site should always be matched in full.
                    return None;
                }
                Some(CodeMapping {
                    span: TextSpan {
                        start: mapping.span.start,
                        end: mapping.span.start,
                    },
                    origin: CodeOrigin::Span(TextSpan {
                        start: TextOffset::default()
                            .add_width(TextWidth::new_for_testing(origin.start.as_u32())),
                        end: TextOffset::default()
                            .add_width(TextWidth::new_for_testing(origin.start.as_u32())),
                    }),
                })
            }
            _ => None,
        })
        .chain(mappings)
        .collect_vec();
    let call_site = TextSpan {
        start: TextOffset::default().add_width(TextWidth::new_for_testing(call_site.start)),
        end: TextOffset::default().add_width(TextWidth::new_for_testing(call_site.end)),
    };
    mappings.push(CodeMapping {
        span: TextSpan {
            start: TextOffset::default(),
            end: mappings.last().map(|m| m.span.end).unwrap_or_default(),
        },
        origin: CodeOrigin::CallSite(call_site),
    });
    mappings
}
