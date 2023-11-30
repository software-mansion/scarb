use anyhow::Result;
use cairo_lang_semantic::plugin::PluginSuite;
use cairo_lang_starknet::starknet_plugin_suite;
use cairo_lang_test_plugin::test_plugin_suite;

use crate::compiler::plugin::{CairoPlugin, CairoPluginInstance};
use crate::core::{PackageId, PackageName, SourceId};
use crate::internal::to_version::ToVersion;

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
    fn plugin_suite(&self) -> PluginSuite {
        starknet_plugin_suite()
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
    fn plugin_suite(&self) -> PluginSuite {
        test_plugin_suite()
    }
}
