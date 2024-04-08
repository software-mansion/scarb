use crate::compiler::plugin::proc_macro::{Expansion, FromSyntaxNode, ProcMacroInstance};
use crate::core::{Config, Package, PackageId};
use anyhow::{ensure, Result};
use cairo_lang_defs::ids::{ModuleItemId, TopLevelLanguageElementId};
use cairo_lang_defs::plugin::{
    DynGeneratedFileAuxData, GeneratedFileAuxData, MacroPlugin, MacroPluginMetadata,
    PluginGeneratedFile, PluginResult,
};
use cairo_lang_defs::plugin::{InlineMacroExprPlugin, InlinePluginResult, PluginDiagnostic};
use cairo_lang_diagnostics::ToOption;
use cairo_lang_macro::{
    AuxData, Diagnostic, ExpansionKind, FullPathMarker, ProcMacroResult, Severity, TokenStream,
    TokenStreamMetadata,
};
use cairo_lang_semantic::db::SemanticGroup;
use cairo_lang_semantic::items::attribute::SemanticQueryAttrs;
use cairo_lang_semantic::plugin::PluginSuite;
use cairo_lang_syntax::attribute::structured::{
    Attribute, AttributeArgVariant, AttributeListStructurize,
};
use cairo_lang_syntax::node::ast::Expr;
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::ids::SyntaxStablePtrId;
use cairo_lang_syntax::node::{ast, TypedStablePtr, TypedSyntaxNode};
use itertools::Itertools;
use scarb_stable_hash::short_hash;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::vec::IntoIter;
use tracing::{debug, trace_span};

const FULL_PATH_MARKER_KEY: &str = "macro::full_path_marker";

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

#[derive(Clone, Debug, Eq, PartialEq)]
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

#[derive(Debug, Clone, Default)]
pub struct EmittedAuxData(Vec<ProcMacroAuxData>);

impl GeneratedFileAuxData for EmittedAuxData {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq(&self, other: &dyn GeneratedFileAuxData) -> bool {
        self.0 == other.as_any().downcast_ref::<Self>().unwrap().0
    }
}

impl EmittedAuxData {
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
        expansions.sort_unstable_by_key(|e| e.expansion.name.clone());
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

    /// Handle `#[proc_macro_name]` attribute.
    fn handle_attribute(
        &self,
        db: &dyn SyntaxGroup,
        item_ast: ast::ModuleItem,
    ) -> Vec<ProcMacroId> {
        let attrs = match item_ast {
            ast::ModuleItem::Struct(struct_ast) => Some(struct_ast.attributes(db)),
            ast::ModuleItem::Enum(enum_ast) => Some(enum_ast.attributes(db)),
            ast::ModuleItem::ExternType(extern_type_ast) => Some(extern_type_ast.attributes(db)),
            ast::ModuleItem::ExternFunction(extern_func_ast) => {
                Some(extern_func_ast.attributes(db))
            }
            ast::ModuleItem::FreeFunction(free_func_ast) => Some(free_func_ast.attributes(db)),
            _ => None,
        };

        attrs
            .map(|attrs| attrs.structurize(db))
            .unwrap_or_default()
            .iter()
            .filter_map(|attr| {
                self.find_expansion(&Expansion::new(attr.id.clone(), ExpansionKind::Attr))
            })
            .collect_vec()
    }

    /// Handle `#[derive(...)]` attribute.
    fn handle_derive(&self, _db: &dyn SyntaxGroup, _item_ast: ast::ModuleItem) -> Vec<ProcMacroId> {
        // Todo(maciektr): Implement.
        Vec::new()
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

    #[tracing::instrument(level = "trace", skip_all)]
    pub fn post_process(&self, db: &dyn SemanticGroup) -> Result<()> {
        let markers = self.collect_full_path_markers(db);

        let aux_data = self.collect_aux_data(db);
        for instance in self.macros.iter() {
            let _ = trace_span!(
                "post_process_callback",
                instance = %instance.package_id()
            )
            .entered();
            let instance_markers = self
                .full_path_markers
                .read()
                .unwrap()
                .get(&instance.package_id())
                .cloned()
                .unwrap_or_default();
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
            instance.post_process_callback(data.clone(), markers_for_instance);
        }
        Ok(())
    }

    fn collect_full_path_markers(&self, db: &dyn SemanticGroup) -> HashMap<String, String> {
        let mut markers: HashMap<String, String> = HashMap::new();
        // FULL_PATH_MARKER_KEY
        for crate_id in db.crates() {
            let modules = db.crate_modules(crate_id);
            for module_id in modules.iter() {
                let Ok(module_items) = db.module_items(*module_id) else {
                    continue;
                };
                for item_id in module_items.iter() {
                    let attr = match item_id {
                        ModuleItemId::Struct(id) => {
                            id.query_attr(db, FULL_PATH_MARKER_KEY).to_option()
                        }
                        ModuleItemId::Enum(id) => {
                            id.query_attr(db, FULL_PATH_MARKER_KEY).to_option()
                        }
                        ModuleItemId::FreeFunction(id) => {
                            id.query_attr(db, FULL_PATH_MARKER_KEY).to_option()
                        }
                        _ => None,
                    };

                    let keys = attr
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|attr| Self::extract_key(db, attr))
                        .collect_vec();
                    let full_path = item_id.full_path(db.upcast());
                    for key in keys {
                        markers.insert(key, full_path.clone());
                    }
                }
            }
        }
        markers
    }

    fn extract_key(db: &dyn SemanticGroup, attr: Attribute) -> Option<String> {
        if attr.id != FULL_PATH_MARKER_KEY {
            return None;
        }

        for arg in attr.args.clone() {
            if let AttributeArgVariant::Unnamed {
                value: Expr::String(s),
                ..
            } = arg.variant
            {
                return s.string_value(db.upcast());
            }
        }

        None
    }

    fn collect_aux_data(
        &self,
        db: &dyn SemanticGroup,
    ) -> HashMap<PackageId, Vec<ProcMacroAuxData>> {
        let mut data = Vec::new();
        for crate_id in db.crates() {
            let crate_modules = db.crate_modules(crate_id);
            for module in crate_modules.iter() {
                let file_infos = db.module_generated_file_infos(*module);
                if let Ok(file_infos) = file_infos {
                    for file_info in file_infos.iter().flatten() {
                        let aux_data = file_info
                            .aux_data
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
        self.macros
            .iter()
            .find(|m| m.package_id() == package_id)
            .expect("procedural macro must be registered in proc macro host")
    }
}

impl MacroPlugin for ProcMacroHostPlugin {
    fn generate_code(
        &self,
        db: &dyn SyntaxGroup,
        item_ast: ast::ModuleItem,
        _metadata: &MacroPluginMetadata<'_>,
    ) -> PluginResult {
        // Apply expansion to `item_ast` where needed.
        let expansions = self
            .handle_attribute(db, item_ast.clone())
            .into_iter()
            .chain(self.handle_derive(db, item_ast.clone()));
        let stable_ptr = item_ast.clone().stable_ptr().untyped();
        let file_path = stable_ptr.file_id(db).full_path(db.upcast());
        let file_id = short_hash(file_path.clone());

        let mut token_stream = TokenStream::from_item_ast(db, item_ast)
            .with_metadata(TokenStreamMetadata::new(file_path, file_id));
        let mut aux_data = EmittedAuxData::default();
        let mut modified = false;
        let mut all_diagnostics: Vec<Diagnostic> = Vec::new();
        for input in expansions {
            match self
                .instance(input.package_id)
                .generate_code(input.expansion.name.clone(), token_stream.clone())
            {
                ProcMacroResult::Replace {
                    token_stream: new_token_stream,
                    aux_data: new_aux_data,
                    diagnostics,
                    full_path_markers,
                } => {
                    token_stream = new_token_stream;
                    if let Some(new_aux_data) = new_aux_data {
                        aux_data.push(ProcMacroAuxData::new(
                            new_aux_data.into(),
                            ProcMacroId::new(input.package_id, input.expansion.clone()),
                        ));
                    }
                    modified = true;
                    all_diagnostics.extend(diagnostics);
                    self.full_path_markers
                        .write()
                        .unwrap()
                        .entry(input.package_id)
                        .and_modify(|markers| markers.extend(full_path_markers.clone().into_iter()))
                        .or_insert(full_path_markers);
                }
                ProcMacroResult::Remove { diagnostics } => {
                    all_diagnostics.extend(diagnostics);
                    return PluginResult {
                        diagnostics: into_cairo_diagnostics(all_diagnostics, stable_ptr),
                        code: None,
                        remove_original_item: true,
                    };
                }
                ProcMacroResult::Leave { diagnostics } => {
                    all_diagnostics.extend(diagnostics);
                }
            };
        }
        if modified {
            PluginResult {
                code: Some(PluginGeneratedFile {
                    name: "proc_macro".into(),
                    content: token_stream.to_string(),
                    code_mappings: Default::default(),
                    aux_data: if aux_data.is_empty() {
                        None
                    } else {
                        Some(DynGeneratedFileAuxData::new(aux_data))
                    },
                }),
                diagnostics: into_cairo_diagnostics(all_diagnostics, stable_ptr),
                remove_original_item: true,
            }
        } else {
            PluginResult {
                code: None,
                diagnostics: into_cairo_diagnostics(all_diagnostics, stable_ptr),
                remove_original_item: false,
            }
        }
    }

    fn declared_attributes(&self) -> Vec<String> {
        self.macros
            .iter()
            .flat_map(|m| m.declared_attributes())
            .chain(vec![FULL_PATH_MARKER_KEY.to_string()])
            .collect()
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
}

impl ProcMacroInlinePlugin {
    pub fn new(instance: Arc<ProcMacroInstance>, expansion: Expansion) -> Self {
        assert!(instance.get_expansions().contains(&expansion));
        Self {
            instance,
            expansion,
        }
    }

    pub fn name(&self) -> &str {
        self.expansion.name.as_str()
    }

    fn instance(&self) -> &ProcMacroInstance {
        &self.instance
    }
}

impl InlineMacroExprPlugin for ProcMacroInlinePlugin {
    fn generate_code(
        &self,
        db: &dyn SyntaxGroup,
        syntax: &ast::ExprInlineMacro,
    ) -> InlinePluginResult {
        let stable_ptr = syntax.clone().stable_ptr().untyped();

        let token_stream = TokenStream::from_syntax_node(db, syntax.as_syntax_node());
        match self
            .instance()
            .generate_code(self.expansion.name.clone(), token_stream)
        {
            ProcMacroResult::Replace {
                token_stream,
                aux_data,
                diagnostics,
                // Ignore, as we know that inline macros do not generate full path markers.
                full_path_markers: _,
            } => {
                let aux_data = aux_data.map(|aux_data| {
                    let aux_data = ProcMacroAuxData::new(
                        aux_data.into(),
                        ProcMacroId::new(self.instance.package_id(), self.expansion.clone()),
                    );
                    let mut emitted = EmittedAuxData::default();
                    emitted.push(aux_data);
                    DynGeneratedFileAuxData::new(emitted)
                });

                InlinePluginResult {
                    code: Some(PluginGeneratedFile {
                        name: "inline_proc_macro".into(),
                        content: token_stream.to_string(),
                        code_mappings: Default::default(),
                        aux_data,
                    }),
                    diagnostics: into_cairo_diagnostics(diagnostics, stable_ptr),
                }
            }
            ProcMacroResult::Remove { diagnostics } => InlinePluginResult {
                code: None,
                diagnostics: into_cairo_diagnostics(diagnostics, stable_ptr),
            },
            ProcMacroResult::Leave { .. } => {
                // Safe to panic, as all inline macros should originally return `InlineProcMacroResult`.
                // Which is validated inside the inline macro helper attribute.
                panic!("inline macro cannot return `Leave` result");
            }
        }
    }
}

fn into_cairo_diagnostics(
    diagnostics: Vec<Diagnostic>,
    stable_ptr: SyntaxStablePtrId,
) -> Vec<PluginDiagnostic> {
    diagnostics
        .into_iter()
        .map(|diag| PluginDiagnostic {
            stable_ptr,
            message: diag.message,
            severity: match diag.severity {
                Severity::Error => cairo_lang_diagnostics::Severity::Error,
                Severity::Warning => cairo_lang_diagnostics::Severity::Warning,
            },
        })
        .collect_vec()
}

/// A Scarb wrapper around the `ProcMacroHost` compiler plugin.
///
/// This struct represent the compiler plugin in terms of Scarb data model.
/// It also builds a plugin suite that enables the compiler plugin.
#[derive(Default)]
pub struct ProcMacroHost {
    macros: Vec<Arc<ProcMacroInstance>>,
}

impl ProcMacroHost {
    pub fn register(&mut self, package: Package, config: &Config) -> Result<()> {
        let instance = ProcMacroInstance::try_new(package, config)?;
        self.macros.push(Arc::new(instance));
        Ok(())
    }

    pub fn into_plugin(self) -> Result<ProcMacroHostPlugin> {
        ProcMacroHostPlugin::try_new(self.macros)
    }
}
