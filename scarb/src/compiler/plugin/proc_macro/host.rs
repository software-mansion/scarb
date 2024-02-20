use crate::compiler::plugin::proc_macro::{FromItemAst, ProcMacroInstance};
use crate::core::{Package, PackageId};
use anyhow::Result;
use cairo_lang_defs::plugin::{
    MacroPlugin, MacroPluginMetadata, PluginGeneratedFile, PluginResult,
};
use cairo_lang_macro::{ProcMacroResult, TokenStream};
use cairo_lang_semantic::plugin::PluginSuite;
use cairo_lang_syntax::attribute::structured::AttributeListStructurize;
use cairo_lang_syntax::node::ast;
use cairo_lang_syntax::node::db::SyntaxGroup;
use itertools::Itertools;
use smol_str::SmolStr;
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

        let mut token_stream = TokenStream::from_item_ast(db, item_ast);
        let mut modified = false;
        for input in expansions {
            let instance = self
                .macros
                .iter()
                .find(|m| m.package_id() == input.macro_package_id)
                .expect("procedural macro must be registered in proc macro host");
            match instance.generate_code(token_stream.clone()) {
                ProcMacroResult::Replace(new_token_stream) => {
                    token_stream = new_token_stream;
                    modified = true;
                }
                ProcMacroResult::Remove => {
                    return PluginResult {
                        code: None,
                        diagnostics: Vec::new(),
                        remove_original_item: true,
                    }
                }
                ProcMacroResult::Leave => {}
            };
        }
        if modified {
            PluginResult {
                code: Some(PluginGeneratedFile {
                    name: "proc_macro".into(),
                    content: token_stream.to_string(),
                    code_mappings: Default::default(),
                    aux_data: Default::default(),
                }),
                diagnostics: Vec::new(),
                remove_original_item: true,
            }
        } else {
            PluginResult::default()
        }
    }

    fn declared_attributes(&self) -> Vec<String> {
        self.macros
            .iter()
            .flat_map(|m| m.declared_attributes())
            .collect()
    }
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
    pub fn register(&mut self, package: Package) -> Result<()> {
        // Create instance
        // Register instance in hash map
        let instance = ProcMacroInstance::try_new(package)?;
        self.macros.push(Arc::new(instance));
        Ok(())
    }

    pub fn into_plugin_suite(self) -> PluginSuite {
        let macro_host = ProcMacroHostPlugin::new(self.macros);
        let mut suite = PluginSuite::default();
        suite.add_plugin_ex(Arc::new(macro_host));
        suite
    }
}
