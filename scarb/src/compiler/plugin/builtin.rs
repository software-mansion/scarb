use anyhow::Result;
use cairo_lang_defs::plugin::{MacroPlugin, MacroPluginMetadata, PluginResult};
use cairo_lang_semantic::plugin::PluginSuite;
use cairo_lang_starknet::starknet_plugin_suite;
use cairo_lang_syntax::node::ast::ModuleItem;
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_test_plugin::{test_assert_suite, test_plugin_suite};

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

pub struct BuiltinCairoRunPlugin;

impl CairoPlugin for BuiltinCairoRunPlugin {
    fn id(&self) -> PackageId {
        PackageId::new(
            PackageName::CAIRO_RUN_PLUGIN,
            crate::version::get().cairo.version.to_version().unwrap(),
            SourceId::for_std(),
        )
    }

    fn instantiate(&self) -> Result<Box<dyn CairoPluginInstance>> {
        Ok(Box::new(BuiltinCairoRunPluginInstance))
    }
}

struct BuiltinCairoRunPluginInstance;

impl CairoPluginInstance for BuiltinCairoRunPluginInstance {
    fn plugin_suite(&self) -> PluginSuite {
        let mut suite = PluginSuite::default();
        suite.add_plugin::<CairoRunPlugin>();
        suite
    }
}

const CAIRO_RUN_EXECUTABLE: &str = "main";

/// A plugin that defines an executable attribute for cairo-run.
/// No code generation is performed.
#[derive(Debug, Default)]
struct CairoRunPlugin {}

impl MacroPlugin for CairoRunPlugin {
    fn generate_code(
        &self,
        _db: &dyn SyntaxGroup,
        _item_ast: ModuleItem,
        _metadata: &MacroPluginMetadata<'_>,
    ) -> PluginResult {
        PluginResult::default()
    }
    fn declared_attributes(&self) -> Vec<String> {
        vec![CAIRO_RUN_EXECUTABLE.to_string()]
    }
    fn executable_attributes(&self) -> Vec<String> {
        self.declared_attributes()
    }
}

pub struct BuiltinTestAssertsPlugin;

impl CairoPlugin for BuiltinTestAssertsPlugin {
    fn id(&self) -> PackageId {
        PackageId::new(
            PackageName::TEST_ASSERTS_PLUGIN,
            semver::Version::new(0, 1, 0),
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
