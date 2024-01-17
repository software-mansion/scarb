use crate::compiler::plugin::proc_macro::{FromItemAst, ProcMacroInstance};
use crate::core::Package;
use anyhow::Result;
use cairo_lang_defs::plugin::{MacroPlugin, MacroPluginMetadata, PluginResult};
use cairo_lang_semantic::plugin::PluginSuite;
use cairo_lang_syntax::node::ast::ModuleItem;
use cairo_lang_syntax::node::db::SyntaxGroup;
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

impl MacroPlugin for ProcMacroHost {
    fn generate_code(
        &self,
        _db: &dyn SyntaxGroup,
        _item_ast: ModuleItem,
        _metadata: &MacroPluginMetadata<'_>,
    ) -> PluginResult {
        // Apply expansion to `item_ast` where needed.
        // TODO(maciektr): Implement
        PluginResult::default()
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
