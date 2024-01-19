use crate::compiler::plugin::proc_macro::{FromItemAst, ProcMacroInstance};
use crate::core::Package;
use anyhow::Result;
use cairo_lang_defs::plugin::{
    MacroPlugin, MacroPluginMetadata, PluginGeneratedFile, PluginResult,
};
use cairo_lang_semantic::plugin::PluginSuite;
use cairo_lang_syntax::attribute::structured::AttributeListStructurize;
use cairo_lang_syntax::node::ast;
use cairo_lang_syntax::node::db::SyntaxGroup;
use itertools::Itertools;
use scarb_macro_interface::{ProcMacroResult, TokenStream};
use smol_str::SmolStr;
use std::collections::HashMap;
use std::sync::Arc;
use typed_builder::TypedBuilder;

/// A Cairo compiler plugin controlling the procedural macro execution.
///
/// This plugin decides which macro plugins (if any) should be applied to the processed AST item.
/// It then redirects the item to the appropriate macro plugin for code expansion.
#[derive(Debug, TypedBuilder)]
pub struct ProcMacroHost {
    macros: HashMap<SmolStr, Arc<ProcMacroInstance>>,
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
}

impl ProcMacroHost {
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
            .filter(|attr| self.macros.contains_key(&attr.id))
            .map(|attr| ProcMacroInput {
                id: attr.id.clone(),
                kind: ProcMacroKind::Attribute,
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
}

impl MacroPlugin for ProcMacroHost {
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
            let instance = self.macros.get(&input.id).unwrap();
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
        self.macros.keys().map(|name| name.to_string()).collect()
    }
}

/// A Scarb wrapper around the `ProcMacroHost` compiler plugin.
///
/// This struct represent the compiler plugin in terms of Scarb data model.
/// It also builds a plugin suite that enables the compiler plugin.
#[derive(Default)]
pub struct ProcMacroHostPlugin {
    macros: HashMap<SmolStr, Arc<ProcMacroInstance>>,
}

impl ProcMacroHostPlugin {
    pub fn register(&mut self, package: Package) -> Result<()> {
        // Create instance
        // Register instance in hash map
        let name = package.id.name.to_smol_str();
        let instance = ProcMacroInstance::try_new(package)?;
        self.macros.insert(name, Arc::new(instance));
        Ok(())
    }

    pub fn plugin_suite(&self) -> PluginSuite {
        let macro_host = ProcMacroHost::builder().macros(self.macros.clone()).build();
        let mut suite = PluginSuite::default();
        suite.add_plugin_ex(Arc::new(macro_host));
        suite
    }
}
