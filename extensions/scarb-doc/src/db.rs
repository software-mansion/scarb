use cairo_lang_compiler::project::{ProjectConfig, update_crate_roots_from_project_config};
use cairo_lang_defs::db::{DefsDatabase, DefsGroup, init_defs_group, try_ext_as_virtual_impl};
use cairo_lang_doc::db::{DocDatabase, DocGroup};
use cairo_lang_filesystem::cfg::{Cfg, CfgSet};
use cairo_lang_filesystem::db::{ExternalFiles, FilesDatabase, FilesGroup, init_files_group};
use cairo_lang_filesystem::ids::{CrateLongId, VirtualFile};
use cairo_lang_lowering::db::{LoweringDatabase, LoweringGroup};
use cairo_lang_parser::db::{ParserDatabase, ParserGroup};
use cairo_lang_semantic::db::{
    PluginSuiteInput, SemanticDatabase, SemanticGroup, init_semantic_group,
};
use cairo_lang_semantic::inline_macros::get_default_plugin_suite;
use cairo_lang_semantic::plugin::PluginSuite;
use cairo_lang_starknet::starknet_plugin_suite;
use cairo_lang_syntax::node::db::{SyntaxDatabase, SyntaxGroup};
use cairo_lang_utils::Upcast;

use salsa;
use scarb_metadata::CompilationUnitComponentMetadata;
use smol_str::ToSmolStr;

/// The Cairo compiler Salsa database tailored for scarb-doc usage.
#[salsa::database(
    FilesDatabase,
    ParserDatabase,
    SyntaxDatabase,
    DefsDatabase,
    SemanticDatabase,
    DocDatabase,
    LoweringDatabase
)]
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

        let interned_plugin_suite = db.intern_plugin_suite(get_default_plugin_suite());
        db.set_default_plugins_from_suite(interned_plugin_suite);

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
            let crate_id = self.intern_crate(CrateLongId::Real {
                name: component.name.to_smolstr(),
                discriminator: component.discriminator.as_ref().map(ToSmolStr::to_smolstr),
            });
            let plugin_suite = [get_default_plugin_suite(), starknet_plugin_suite()]
                .into_iter()
                .fold(PluginSuite::default(), |mut acc, suite| {
                    acc.add(suite);
                    acc
                });
            let interned_suite = self.intern_plugin_suite(plugin_suite);
            self.set_override_crate_plugins_from_suite(crate_id, interned_suite);
        }
    }
}

impl salsa::Database for ScarbDocDatabase {}

impl ExternalFiles for ScarbDocDatabase {
    fn try_ext_as_virtual(&self, external_id: salsa::InternId) -> Option<VirtualFile> {
        try_ext_as_virtual_impl(self.upcast(), external_id)
    }
}

impl salsa::ParallelDatabase for ScarbDocDatabase {
    fn snapshot(&self) -> salsa::Snapshot<Self> {
        salsa::Snapshot::new(ScarbDocDatabase {
            storage: self.storage.snapshot(),
        })
    }
}

impl Upcast<dyn FilesGroup> for ScarbDocDatabase {
    fn upcast(&self) -> &(dyn FilesGroup + 'static) {
        self
    }
}

impl Upcast<dyn ParserGroup> for ScarbDocDatabase {
    fn upcast(&self) -> &(dyn ParserGroup + 'static) {
        self
    }
}

impl Upcast<dyn SyntaxGroup> for ScarbDocDatabase {
    fn upcast(&self) -> &(dyn SyntaxGroup + 'static) {
        self
    }
}

impl Upcast<dyn DefsGroup> for ScarbDocDatabase {
    fn upcast(&self) -> &(dyn DefsGroup + 'static) {
        self
    }
}

impl Upcast<dyn SemanticGroup> for ScarbDocDatabase {
    fn upcast(&self) -> &(dyn SemanticGroup + 'static) {
        self
    }
}

impl Upcast<dyn DocGroup> for ScarbDocDatabase {
    fn upcast(&self) -> &(dyn DocGroup + 'static) {
        self
    }
}

impl Upcast<dyn LoweringGroup> for ScarbDocDatabase {
    fn upcast(&self) -> &(dyn LoweringGroup + 'static) {
        self
    }
}
