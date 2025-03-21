mod attribute;
mod aux_data;
mod conversion;
mod derive;
mod inline;
mod post;

use attribute::*;
pub use aux_data::ProcMacroAuxData;
use inline::*;

use crate::compiler::plugin::proc_macro::ProcMacroInstance;
use crate::compiler::plugin::proc_macro::expansion::{Expansion, ExpansionKind};
use crate::core::{PackageId, edition_variant};
use anyhow::{Result, ensure};
use cairo_lang_defs::plugin::{MacroPlugin, MacroPluginMetadata, PluginResult};
use cairo_lang_filesystem::db::Edition;
use cairo_lang_filesystem::ids::{CodeMapping, CodeOrigin};
use cairo_lang_filesystem::span::{TextOffset, TextSpan, TextWidth};
use cairo_lang_macro::{AllocationContext, TokenStream, TokenStreamMetadata, TokenTree};
use cairo_lang_semantic::plugin::PluginSuite;
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::{TypedStablePtr, TypedSyntaxNode, ast};
use convert_case::{Case, Casing};
use itertools::Itertools;
use scarb_stable_hash::short_hash;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};

const FULL_PATH_MARKER_KEY: &str = "macro::full_path_marker";
const DERIVE_ATTR: &str = "derive";

/// A Cairo compiler plugin controlling the procedural macro execution.
///
/// This plugin decides which macro plugins (if any) should be applied to the processed AST item.
/// It then redirects the item to the appropriate macro plugin for code expansion.
#[derive(Debug)]
pub struct ProcMacroHostPlugin {
    macros: Vec<Arc<ProcMacroInstance>>,
    full_path_markers: RwLock<HashMap<PackageId, Vec<String>>>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
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
        expansions.sort_unstable_by_key(|e| (e.expansion.name.clone(), e.package_id));
        ensure!(
            expansions
                .windows(2)
                .all(|w| w[0].expansion.name != w[1].expansion.name),
            "duplicate expansions defined for procedural macros: {duplicates}",
            duplicates = expansions
                .windows(2)
                .filter(|w| w[0].expansion.name == w[1].expansion.name)
                .map(|w| format!(
                    "{} ({} and {})",
                    w[0].expansion.name.as_str(),
                    w[0].package_id,
                    w[1].package_id
                ))
                .collect::<Vec<_>>()
                .join(", ")
        );
        Ok(Self {
            macros,
            full_path_markers: RwLock::new(Default::default()),
        })
    }

    fn find_expansion(&self, expansion: &Expansion) -> Option<ProcMacroId> {
        self.macros
            .iter()
            .find(|m| m.get_expansions().contains(expansion))
            .map(|m| m.package_id())
            .map(|package_id| ProcMacroId::new(package_id, expansion.clone()))
    }

    pub fn build_plugin_suite(macro_host: Arc<Self>) -> PluginSuite {
        let mut suite = PluginSuite::default();
        // Register inline macro plugins.
        for proc_macro in &macro_host.macros {
            let expansions = proc_macro
                .get_expansions()
                .iter()
                .filter(|exp| matches!(exp.kind, ExpansionKind::Inline));
            for expansion in expansions {
                let plugin = Arc::new(ProcMacroInlinePlugin::new(
                    proc_macro.clone(),
                    expansion.clone(),
                ));
                suite.add_inline_macro_plugin_ex(expansion.name.as_str(), plugin);
            }
        }
        // Register procedural macro host plugin.
        suite.add_plugin_ex(macro_host);
        suite
    }

    pub fn instance(&self, package_id: PackageId) -> &ProcMacroInstance {
        self.macros
            .iter()
            .find(|m| m.package_id() == package_id)
            .expect("procedural macro must be registered in proc macro host")
    }

    fn calculate_metadata(
        db: &dyn SyntaxGroup,
        item_ast: ast::ModuleItem,
        edition: Edition,
    ) -> TokenStreamMetadata {
        let stable_ptr = item_ast.clone().stable_ptr().untyped();
        let file_path = stable_ptr.file_id(db).full_path(db.upcast());
        let file_id = short_hash(file_path.clone());
        let edition = edition_variant(edition);
        TokenStreamMetadata::new(file_path, file_id, edition)
    }

    pub fn macros(&self) -> &[Arc<ProcMacroInstance>] {
        &self.macros
    }

    // NOTE: Required for proc macro server. `<ProcMacroHostPlugin as MacroPlugin>::declared_attributes`
    // returns attributes **and** executables. In PMS, we only need the former because the latter is handled separately.
    pub fn declared_attributes_without_executables(&self) -> Vec<String> {
        self.macros
            .iter()
            .flat_map(|instance| instance.declared_attributes())
            .collect()
    }

    pub fn declared_inline_macros(&self) -> Vec<String> {
        self.macros
            .iter()
            .flat_map(|instance| instance.inline_macros())
            .collect()
    }
}

impl MacroPlugin for ProcMacroHostPlugin {
    fn generate_code(
        &self,
        db: &dyn SyntaxGroup,
        item_ast: ast::ModuleItem,
        metadata: &MacroPluginMetadata<'_>,
    ) -> PluginResult {
        let stream_metadata = Self::calculate_metadata(db, item_ast.clone(), metadata.edition);

        // Handle inner functions.
        if let InnerAttrExpansionResult::Some(result) = self.expand_inner_attr(db, item_ast.clone())
        {
            return result;
        }

        // Expand first attribute.
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
            self.expand_attribute(input.id, last, input.args, token_stream, input.call_site)
        }) {
            return result;
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

    fn declared_attributes(&self) -> Vec<String> {
        self.macros
            .iter()
            .flat_map(|m| m.declared_attributes_and_executables())
            .chain(vec![FULL_PATH_MARKER_KEY.to_string()])
            .collect()
    }

    fn declared_derives(&self) -> Vec<String> {
        self.macros
            .iter()
            .flat_map(|m| m.declared_derives())
            .map(|s| s.to_case(Case::UpperCamel))
            .collect()
    }

    fn executable_attributes(&self) -> Vec<String> {
        self.macros
            .iter()
            .flat_map(|m| m.executable_attributes())
            .collect()
    }
}

fn generate_code_mappings(token_stream: &TokenStream) -> Vec<CodeMapping> {
    token_stream
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
        .collect()
}
