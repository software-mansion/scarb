use std::sync::Arc;

use anyhow::Result;
use cairo_lang_defs::plugin::{InlineMacroExprPlugin, MacroPlugin};
use cairo_lang_starknet::inline_macros::selector::SelectorMacro;
use cairo_lang_starknet::plugin::StarkNetPlugin;
use cairo_lang_test_plugin::TestPlugin;

use crate::compiler::plugin::{CairoPlugin, CairoPluginInstance};
use crate::core::{PackageId, PackageName, SourceId};
use crate::internal::to_version::ToVersion;

impl CairoPluginInstance for Arc<dyn MacroPlugin> {
    fn macro_plugins(&self) -> Vec<Arc<dyn MacroPlugin>> {
        Vec::from_iter([self.clone()])
    }

    fn inline_macro_plugins(&self) -> Vec<(String, Arc<dyn InlineMacroExprPlugin>)> {
        Vec::new()
    }
}

pub struct BuiltinStarkNetPlugin;
impl CairoPlugin for BuiltinStarkNetPlugin {
    fn id(&self) -> PackageId {
        PackageId::new(
            PackageName::STARKNET,
            crate::version::get().cairo.version.to_version().unwrap(),
            SourceId::for_std(),
        )
    }

    fn instantiate(&self) -> Result<Box<dyn CairoPluginInstance>> {
        Ok(Box::new(BuiltinStarkNetPluginInstance))
    }
}

struct BuiltinStarkNetPluginInstance;
impl CairoPluginInstance for BuiltinStarkNetPluginInstance {
    fn macro_plugins(&self) -> Vec<Arc<dyn MacroPlugin>> {
        vec![Arc::new(StarkNetPlugin::default())]
    }

    fn inline_macro_plugins(&self) -> Vec<(String, Arc<dyn InlineMacroExprPlugin>)> {
        vec![(SelectorMacro::NAME.into(), Arc::new(SelectorMacro))]
    }
}

pub struct BuiltinTestPlugin;

impl CairoPlugin for BuiltinTestPlugin {
    fn id(&self) -> PackageId {
        PackageId::new(
            PackageName::TEST_PLUGIN,
            crate::version::get().cairo.version.to_version().unwrap(),
            SourceId::for_std(),
        )
    }

    fn instantiate(&self) -> Result<Box<dyn CairoPluginInstance>> {
        Ok(Box::new(BuiltinTestPluginInstance))
    }
}

struct BuiltinTestPluginInstance;

impl CairoPluginInstance for BuiltinTestPluginInstance {
    fn macro_plugins(&self) -> Vec<Arc<dyn MacroPlugin>> {
        vec![Arc::new(TestPlugin::default())]
    }

    fn inline_macro_plugins(&self) -> Vec<(String, Arc<dyn InlineMacroExprPlugin>)> {
        Vec::new()
    }
}
