use crate::compiler::plugin::proc_macro::ProcMacroInstance;
use crate::core::Package;
use anyhow::Result;
use cairo_lang_defs::plugin::{MacroPlugin, MacroPluginMetadata, PluginResult};
use cairo_lang_semantic::plugin::PluginSuite;
use cairo_lang_syntax::node::ast::ModuleItem;
use cairo_lang_syntax::node::db::SyntaxGroup;
use std::sync::Arc;

/// A Cairo compiler plugin controlling the procedural macro execution.
///
/// This plugin decides which macro plugins (if any) should be applied to the processed AST item.
/// It then redirects the item to the appropriate macro plugin for code expansion.
#[derive(Debug)]
pub struct ProcMacroHostPlugin {
    macros: Vec<Arc<ProcMacroInstance>>,
}

impl ProcMacroHostPlugin {
    pub fn new(macros: Vec<Arc<ProcMacroInstance>>) -> Self {
        Self { macros }
    }
}

impl MacroPlugin for ProcMacroHostPlugin {
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
