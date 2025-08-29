use anyhow::Result;
use cairo_lang_executable_plugin::executable_plugin_suite;
use cairo_lang_semantic::plugin::PluginSuite;
use cairo_lang_starknet::starknet_plugin_suite;
use cairo_lang_test_plugin::{test_assert_suite, test_plugin_suite};

use crate::compiler::plugin::{CairoPlugin, CairoPluginInstance};
use crate::core::{PackageId, PackageName, SourceId};
use crate::internal::to_version::ToVersion;

pub struct BuiltinStarknetPlugin;
impl CairoPlugin for BuiltinStarknetPlugin {
    fn id(&self) -> PackageId {
        PackageId::new(
            PackageName::STARKNET,
            crate::version::get().cairo.version.to_version().unwrap(),
            SourceId::for_std(),
        )
    }

    fn instantiate(&self) -> Result<Box<dyn CairoPluginInstance>> {
        Ok(Box::new(BuiltinStarknetPluginInstance))
    }
}

struct BuiltinStarknetPluginInstance;
impl CairoPluginInstance for BuiltinStarknetPluginInstance {
    fn plugin_suite(&self) -> PluginSuite {
        starknet_plugin_suite()
    }
}

pub struct BuiltinExecutablePlugin;
impl CairoPlugin for BuiltinExecutablePlugin {
    fn id(&self) -> PackageId {
        PackageId::new(
            PackageName::EXECUTABLE,
            crate::version::get().cairo.version.to_version().unwrap(),
            SourceId::for_std(),
        )
    }

    fn instantiate(&self) -> Result<Box<dyn CairoPluginInstance>> {
        Ok(Box::new(BuiltinExecutablePluginInstance))
    }
}

struct BuiltinExecutablePluginInstance;
impl CairoPluginInstance for BuiltinExecutablePluginInstance {
    fn plugin_suite(&self) -> PluginSuite {
        executable_plugin_suite()
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

pub struct BuiltinTestAssertsPlugin;

impl CairoPlugin for BuiltinTestAssertsPlugin {
    fn id(&self) -> PackageId {
        PackageId::new(
            PackageName::TEST_ASSERTS_PLUGIN,
            crate::version::get().cairo.version.to_version().unwrap(),
            SourceId::for_std(),
        )
    }

    fn instantiate(&self) -> Result<Box<dyn CairoPluginInstance>> {
        Ok(Box::new(BuiltinTestAssertsPluginInstance))
    }
}

struct BuiltinTestAssertsPluginInstance;

impl CairoPluginInstance for BuiltinTestAssertsPluginInstance {
    fn plugin_suite(&self) -> PluginSuite {
        test_assert_suite()
    }
}
