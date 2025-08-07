use cairo_lang_compiler::project::{ProjectConfig, update_crate_roots_from_project_config};
use cairo_lang_defs::db::{DefsGroup, init_defs_group, try_ext_as_virtual_impl};
use cairo_lang_defs::ids::{InlineMacroExprPluginLongId, MacroPluginLongId};
use cairo_lang_doc::db::DocGroup;
use cairo_lang_filesystem::cfg::{Cfg, CfgSet};
use cairo_lang_filesystem::db::{ExternalFiles, FilesGroup, init_files_group};
use cairo_lang_filesystem::ids::{CrateInput, CrateLongId, VirtualFile};
use cairo_lang_lowering::db::{LoweringGroup, UseApproxCodeSizeEstimator};
use cairo_lang_parser::db::ParserGroup;
use cairo_lang_semantic::db::{Elongate, PluginSuiteInput, SemanticGroup, init_semantic_group};
use cairo_lang_semantic::ids::AnalyzerPluginLongId;
use cairo_lang_semantic::inline_macros::get_default_plugin_suite;
use cairo_lang_semantic::plugin::PluginSuite;
use cairo_lang_starknet::starknet_plugin_suite;
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_utils::Upcast;
use std::sync::Arc;

use salsa;
use scarb_metadata::CompilationUnitComponentMetadata;

impl UseApproxCodeSizeEstimator for ScarbDocDatabase {}

/// The Cairo compiler Salsa database tailored for scarb-doc usage.
#[salsa::db]
#[derive(Clone)]
pub struct ScarbDocDatabase {
    storage: salsa::Storage<Self>,
}

impl ScarbDocDatabase {
    pub fn new(
        project_config: ProjectConfig,
        crates_with_starknet: Vec<&CompilationUnitComponentMetadata>,
    ) -> Self {
        let mut db = Self {
            storage: Default::default(),
        };

        init_files_group(&mut db);
        init_defs_group(&mut db);
        init_semantic_group(&mut db);

        db.set_cfg_set(Self::initial_cfg_set().into());

        db.set_default_plugins_from_suite(get_default_plugin_suite());

        db.apply_project_config(project_config);
        db.apply_starknet_plugin(crates_with_starknet);

        db
    }

    fn initial_cfg_set() -> CfgSet {
        CfgSet::from_iter([Cfg::name("doc")])
    }

    fn apply_project_config(&mut self, config: ProjectConfig) {
        update_crate_roots_from_project_config(self, &config);
    }

    fn apply_starknet_plugin(&mut self, components: Vec<&CompilationUnitComponentMetadata>) {
        for component in components {
            let plugin_suite = [get_default_plugin_suite(), starknet_plugin_suite()]
                .into_iter()
                .fold(PluginSuite::default(), |mut acc, suite| {
                    acc.add(suite);
                    acc
                });
            let crate_id = CrateLongId::Real {
                name: component.name.to_string(),
                discriminator: component.discriminator.as_ref().map(ToString::to_string),
            }
            .into_crate_input(self);
            self.set_override_crate_plugins_from_suite(crate_id, plugin_suite);
        }
    }

    pub fn set_override_crate_plugins_from_suite(
        &mut self,
        crate_input: CrateInput,
        plugins: PluginSuite,
    ) {
        let mut overrides = self.macro_plugin_overrides_input().as_ref().clone();
        overrides.insert(
            crate_input.clone(),
            plugins.plugins.into_iter().map(MacroPluginLongId).collect(),
        );
        self.set_macro_plugin_overrides_input(overrides.into());

        let mut overrides = self.analyzer_plugin_overrides_input().as_ref().clone();
        overrides.insert(
            crate_input.clone(),
            plugins
                .analyzer_plugins
                .into_iter()
                .map(AnalyzerPluginLongId)
                .collect(),
        );
        self.set_analyzer_plugin_overrides_input(overrides.into());

        let mut overrides = self.inline_macro_plugin_overrides_input().as_ref().clone();
        overrides.insert(
            crate_input,
            Arc::new(
                plugins
                    .inline_macro_plugins
                    .into_iter()
                    .map(|(key, value)| (key, InlineMacroExprPluginLongId(value)))
                    .collect(),
            ),
        );
        self.set_inline_macro_plugin_overrides_input(overrides.into());
    }
}

impl salsa::Database for ScarbDocDatabase {}

impl ExternalFiles for ScarbDocDatabase {
    fn try_ext_as_virtual(&self, external_id: salsa::Id) -> Option<VirtualFile> {
        try_ext_as_virtual_impl(self.upcast(), external_id)
    }
}

impl<'db> Upcast<'db, dyn FilesGroup> for ScarbDocDatabase {
    fn upcast(&self) -> &(dyn FilesGroup + 'static) {
        self
    }
}

impl<'db> Upcast<'db, dyn ParserGroup> for ScarbDocDatabase {
    fn upcast(&self) -> &(dyn ParserGroup + 'static) {
        self
    }
}

impl<'db> Upcast<'db, dyn SyntaxGroup> for ScarbDocDatabase {
    fn upcast(&self) -> &(dyn SyntaxGroup + 'static) {
        self
    }
}

impl<'db> Upcast<'db, dyn DefsGroup> for ScarbDocDatabase {
    fn upcast(&self) -> &(dyn DefsGroup + 'static) {
        self
    }
}

impl<'db> Upcast<'db, dyn SemanticGroup> for ScarbDocDatabase {
    fn upcast(&self) -> &(dyn SemanticGroup + 'static) {
        self
    }
}

impl<'db> Upcast<'db, dyn DocGroup> for ScarbDocDatabase {
    fn upcast(&self) -> &(dyn DocGroup + 'static) {
        self
    }
}

impl<'db> Upcast<'db, dyn LoweringGroup> for ScarbDocDatabase {
    fn upcast(&self) -> &(dyn LoweringGroup + 'static) {
        self
    }
}

impl Elongate for ScarbDocDatabase {
    fn elongate(&self) -> &(dyn SemanticGroup + 'static) {
        self
    }
}
