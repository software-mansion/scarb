use cairo_lang_compiler::project::{ProjectConfig, update_crate_roots_from_project_config};
use cairo_lang_defs::db::{
    init_defs_group, init_external_files, set_inline_macro_plugin_overrides_for_input,
    set_macro_plugin_overrides_for_input,
};
use cairo_lang_defs::ids::{InlineMacroExprPluginLongId, MacroPluginLongId};
use cairo_lang_filesystem::cfg::{Cfg, CfgSet};
use cairo_lang_filesystem::db::{FilesGroup, init_files_group};
use cairo_lang_filesystem::ids::{CrateInput, CrateLongId, SmolStrId};
use cairo_lang_semantic::db::{
    PluginSuiteInput, SemanticGroup, init_semantic_group, set_analyzer_plugin_overrides_for_input,
};
use cairo_lang_semantic::ids::AnalyzerPluginLongId;
use cairo_lang_semantic::inline_macros::get_default_plugin_suite;
use cairo_lang_semantic::plugin::PluginSuite;
use cairo_lang_starknet::starknet_plugin_suite;
use std::sync::Arc;

use salsa;
use scarb_metadata::CompilationUnitComponentMetadata;

/// The Cairo compiler Salsa database tailored for scarb-doc usage.
#[salsa::db]
#[derive(Clone)]
pub struct ScarbDocDatabase {
    storage: salsa::Storage<Self>,
}

impl Default for ScarbDocDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl ScarbDocDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            storage: Default::default(),
        };

        init_files_group(&mut db);
        init_defs_group(&mut db);
        init_semantic_group(&mut db);
        init_external_files(&mut db);

        db.use_cfg(&Self::initial_cfg_set());
        db.set_default_plugins_from_suite(get_default_plugin_suite());

        db
    }

    fn initial_cfg_set() -> CfgSet {
        CfgSet::from_iter([Cfg::name("doc")])
    }

    pub fn apply_project_config(&mut self, config: ProjectConfig) {
        update_crate_roots_from_project_config(self, &config);
    }

    pub fn apply_starknet_plugin(&mut self, components: Vec<&CompilationUnitComponentMetadata>) {
        for component in components {
            let plugin_suite = [get_default_plugin_suite(), starknet_plugin_suite()]
                .into_iter()
                .fold(PluginSuite::default(), |mut acc, suite| {
                    acc.add(suite);
                    acc
                });
            let crate_input = CrateLongId::Real {
                name: SmolStrId::from(self, component.name.as_str()),
                discriminator: component.discriminator.as_ref().map(ToString::to_string),
            }
            .into_crate_input(self);
            self.set_override_crate_plugins_from_suite(crate_input, plugin_suite);
        }
    }

    pub fn set_override_crate_plugins_from_suite(
        &mut self,
        crate_input: CrateInput,
        plugins: PluginSuite,
    ) {
        set_macro_plugin_overrides_for_input(
            self,
            crate_input.clone(),
            Some(plugins.plugins.into_iter().map(MacroPluginLongId).collect()),
        );

        set_analyzer_plugin_overrides_for_input(
            self,
            crate_input.clone(),
            Some(
                plugins
                    .analyzer_plugins
                    .into_iter()
                    .map(AnalyzerPluginLongId)
                    .collect(),
            ),
        );

        set_inline_macro_plugin_overrides_for_input(
            self,
            crate_input,
            Some(Arc::new(
                plugins
                    .inline_macro_plugins
                    .into_iter()
                    .map(|(key, value)| (key, InlineMacroExprPluginLongId(value)))
                    .collect(),
            )),
        );
    }
}

impl salsa::Database for ScarbDocDatabase {}
