use cairo_lang_compiler::project::{
    update_crate_root, update_crate_roots_from_project_config, ProjectConfig,
};
use cairo_lang_defs::db::{ext_as_virtual_impl, DefsDatabase, DefsGroup};
use cairo_lang_doc::db::{DocDatabase, DocGroup};
use cairo_lang_filesystem::cfg::{Cfg, CfgSet};
use cairo_lang_filesystem::db::{
    init_files_group, AsFilesGroupMut, ExternalFiles, FilesDatabase, FilesGroup, CORELIB_CRATE_NAME,
};
use cairo_lang_filesystem::ids::VirtualFile;
use cairo_lang_lowering::db::{LoweringDatabase, LoweringGroup};
use cairo_lang_parser::db::{ParserDatabase, ParserGroup};
use cairo_lang_semantic::db::{SemanticDatabase, SemanticGroup};
use cairo_lang_semantic::inline_macros::get_default_plugin_suite;
use cairo_lang_semantic::plugin::PluginSuite;
use cairo_lang_starknet::starknet_plugin_suite;
use cairo_lang_syntax::node::db::{SyntaxDatabase, SyntaxGroup};
use cairo_lang_utils::Upcast;

use salsa;
use scarb_metadata::PackageMetadata;
use semver::Version;

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
    main_package_name: String,
    main_package_version: Version,
}

impl ScarbDocDatabase {
    pub fn new(project_config: Option<ProjectConfig>, main_package: &PackageMetadata) -> Self {
        let plugin_suite = [get_default_plugin_suite(), starknet_plugin_suite()]
            .into_iter()
            .fold(PluginSuite::default(), |mut acc, suite| {
                acc.add(suite);
                acc
            });
        let mut db = Self {
            storage: Default::default(),
            main_package_name: main_package.name.clone(),
            main_package_version: main_package.version.clone(),
        };

        init_files_group(&mut db);

        db.set_cfg_set(Self::initial_cfg_set().into());

        db.apply_plugin_suite(plugin_suite);

        if let Some(config) = project_config {
            db.apply_project_config(config);
        }

        db
    }

    fn initial_cfg_set() -> CfgSet {
        CfgSet::from_iter([Cfg::name("doc")])
    }

    fn apply_plugin_suite(&mut self, plugin_suite: PluginSuite) {
        self.set_macro_plugins(plugin_suite.plugins);
        self.set_inline_macro_plugins(plugin_suite.inline_macro_plugins.into());
        self.set_analyzer_plugins(plugin_suite.analyzer_plugins);
    }

    fn apply_project_config(&mut self, config: ProjectConfig) {
        update_crate_roots_from_project_config(self, &config);
        if let Some(corelib) = &config.corelib {
            update_crate_root(self, &config, CORELIB_CRATE_NAME.into(), corelib.clone());
        }
    }
}

impl salsa::Database for ScarbDocDatabase {}

impl ExternalFiles for ScarbDocDatabase {
    fn ext_as_virtual(&self, external_id: salsa::InternId) -> VirtualFile {
        ext_as_virtual_impl(self.upcast(), external_id)
    }
}

impl salsa::ParallelDatabase for ScarbDocDatabase {
    fn snapshot(&self) -> salsa::Snapshot<Self> {
        salsa::Snapshot::new(ScarbDocDatabase {
            storage: self.storage.snapshot(),
            main_package_name: self.main_package_name.clone(),
            main_package_version: self.main_package_version.clone(),
        })
    }
}

impl AsFilesGroupMut for ScarbDocDatabase {
    fn as_files_group_mut(&mut self) -> &mut (dyn FilesGroup + 'static) {
        self
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
