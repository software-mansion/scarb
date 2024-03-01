use crate::compiler::plugin::proc_macro::{FromItemAst, ProcMacroInstance};
use crate::core::{Config, Package, PackageId};
use anyhow::Result;
use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::plugin::PluginDiagnostic;
use cairo_lang_defs::plugin::{
    DynGeneratedFileAuxData, GeneratedFileAuxData, MacroPlugin, MacroPluginMetadata,
    PluginGeneratedFile, PluginResult,
};
use cairo_lang_macro::{AuxData, Diagnostic, ProcMacroResult, Severity, TokenStream};
use cairo_lang_semantic::plugin::PluginSuite;
use cairo_lang_syntax::attribute::structured::AttributeListStructurize;
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::ids::SyntaxStablePtrId;
use cairo_lang_syntax::node::{ast, TypedSyntaxNode};
use itertools::Itertools;
use smol_str::SmolStr;
use std::any::Any;
use std::sync::Arc;

/// A Cairo compiler plugin controlling the procedural macro execution.
///
/// This plugin decides which macro plugins (if any) should be applied to the processed AST item.
/// It then redirects the item to the appropriate macro plugin for code expansion.
#[derive(Debug)]
pub struct ProcMacroHostPlugin {
    macros: Vec<Arc<ProcMacroInstance>>,
}

pub type ProcMacroId = SmolStr;

#[derive(Debug)]
#[allow(dead_code)]
pub enum ProcMacroKind {
    /// `proc_macro_name!(...)`
    MacroCall,
    /// `#[proc_macro_name]`
    Attribute,
    /// `#[derive(...)]`
    Derive,
}

#[derive(Debug)]
pub struct ProcMacroInput {
    pub id: ProcMacroId,
    pub kind: ProcMacroKind,
    pub macro_package_id: PackageId,
}

#[derive(Clone, Debug)]
pub struct ProcMacroAuxData {
    value: String,
    macro_id: ProcMacroId,
    macro_package_id: PackageId,
}

impl ProcMacroAuxData {
    pub fn new(value: String, macro_id: ProcMacroId, macro_package_id: PackageId) -> Self {
        Self {
            value,
            macro_id,
            macro_package_id,
        }
    }
}

impl From<ProcMacroAuxData> for AuxData {
    fn from(data: ProcMacroAuxData) -> Self {
        Self::new(data.value)
    }
}

impl GeneratedFileAuxData for ProcMacroAuxData {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq(&self, other: &dyn GeneratedFileAuxData) -> bool {
        self.value == other.as_any().downcast_ref::<Self>().unwrap().value
            && self.macro_id == other.as_any().downcast_ref::<Self>().unwrap().macro_id
    }
}

impl ProcMacroHostPlugin {
    pub fn new(macros: Vec<Arc<ProcMacroInstance>>) -> Self {
        Self { macros }
    }

    /// Handle `proc_macro_name!` expression.
    fn handle_macro(
        &self,
        _db: &dyn SyntaxGroup,
        _item_ast: ast::ModuleItem,
    ) -> Vec<ProcMacroInput> {
        // Todo(maciektr): Implement.
        Vec::new()
    }

    /// Handle `#[proc_macro_name]` attribute.
    fn handle_attribute(
        &self,
        db: &dyn SyntaxGroup,
        item_ast: ast::ModuleItem,
    ) -> Vec<ProcMacroInput> {
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
                self.find_macro_package(attr.id.to_string())
                    .map(|pid| ProcMacroInput {
                        id: attr.id.clone(),
                        kind: ProcMacroKind::Attribute,
                        macro_package_id: pid,
                    })
            })
            .collect_vec()
    }

    /// Handle `#[derive(...)]` attribute.
    fn handle_derive(
        &self,
        _db: &dyn SyntaxGroup,
        _item_ast: ast::ModuleItem,
    ) -> Vec<ProcMacroInput> {
        // Todo(maciektr): Implement.
        Vec::new()
    }

    fn find_macro_package(&self, name: String) -> Option<PackageId> {
        self.macros
            .iter()
            .find(|m| m.declared_attributes().contains(&name))
            .map(|m| m.package_id())
    }

    pub fn build_plugin_suite(macr_host: Arc<Self>) -> PluginSuite {
        let mut suite = PluginSuite::default();
        suite.add_plugin_ex(macr_host);
        suite
    }

    #[tracing::instrument(level = "trace", skip_all)]
    pub fn collect_aux_data(&self, db: &dyn DefsGroup) -> Result<()> {
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
                            .and_then(|ad| ad.as_any().downcast_ref::<ProcMacroAuxData>());
                        if let Some(aux_data) = aux_data {
                            data.push(aux_data.clone());
                        }
                    }
                }
            }
        }
        let aux_data = data.into_iter().into_group_map_by(|d| d.macro_package_id);
        for instance in self.macros.iter() {
            let data = aux_data.get(&instance.package_id()).cloned();
            if let Some(data) = data {
                instance.aux_data_callback(data.clone());
            }
        }
        Ok(())
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
            .handle_macro(db, item_ast.clone())
            .into_iter()
            .chain(self.handle_attribute(db, item_ast.clone()))
            .chain(self.handle_derive(db, item_ast.clone()));
        let stable_ptr = item_ast.clone().stable_ptr().untyped();

        let mut token_stream = TokenStream::from_item_ast(db, item_ast);
        let mut aux_data: Option<ProcMacroAuxData> = None;
        let mut modified = false;
        let mut all_diagnostics: Vec<Diagnostic> = Vec::new();
        for input in expansions {
            let instance = self
                .macros
                .iter()
                .find(|m| m.package_id() == input.macro_package_id)
                .expect("procedural macro must be registered in proc macro host");
            match instance.generate_code(token_stream.clone()) {
                ProcMacroResult::Replace {
                    token_stream: new_token_stream,
                    aux_data: new_aux_data,
                    diagnostics,
                } => {
                    token_stream = new_token_stream;
                    if let Some(new_aux_data) = new_aux_data {
                        aux_data = Some(ProcMacroAuxData::new(
                            new_aux_data.to_string(),
                            input.id,
                            input.macro_package_id,
                        ));
                    }
                    modified = true;
                    all_diagnostics.extend(diagnostics);
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
                    aux_data: aux_data.map(DynGeneratedFileAuxData::new),
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
            .collect()
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

    pub fn into_plugin(self) -> ProcMacroHostPlugin {
        ProcMacroHostPlugin::new(self.macros)
    }
}
