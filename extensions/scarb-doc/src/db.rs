use anyhow::Result;
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::diagnostics::DiagnosticsReporter;
use cairo_lang_compiler::project::{
    update_crate_root, update_crate_roots_from_project_config, ProjectConfig,
};
use cairo_lang_defs::db::{ext_as_virtual_impl, DefsDatabase, DefsGroup};
use cairo_lang_diagnostics::{FormattedDiagnosticEntry, Severity};
use cairo_lang_doc::db::{DocDatabase, DocGroup};
use cairo_lang_filesystem::cfg::{Cfg, CfgSet};
use cairo_lang_filesystem::db::{
    init_files_group, AsFilesGroupMut, ExternalFiles, FilesDatabase, FilesGroup, CORELIB_CRATE_NAME,
};
use cairo_lang_filesystem::ids::VirtualFile;
use cairo_lang_parser::db::{ParserDatabase, ParserGroup};
use cairo_lang_semantic::db::{SemanticDatabase, SemanticGroup};
use cairo_lang_semantic::inline_macros::get_default_plugin_suite;
use cairo_lang_semantic::plugin::PluginSuite;
use cairo_lang_starknet::starknet_plugin_suite;
use cairo_lang_syntax::node::db::{SyntaxDatabase, SyntaxGroup};
use cairo_lang_utils::Upcast;

use salsa;

/// The Cairo compiler Salsa database tailored for scarb-doc usage.
// #[salsa::database(
//     FilesDatabase,
//     ParserDatabase,
//     SyntaxDatabase,
//     DefsDatabase,
//     SemanticDatabase,
//     DocDatabase
// )]
pub struct ScarbDocDatabase {
    // storage: salsa::Storage<Self>,
    pub db: RootDatabase,
}

impl ScarbDocDatabase {
    pub fn new(project_config: Option<ProjectConfig>) -> Result<Self> {
        let mut db_builder = RootDatabase::builder();
        if let Some(config) = project_config {
            db_builder.with_project_config(config);
        }
        db_builder.with_cfg(Self::initial_cfg_set());

        let plugin_suite = [get_default_plugin_suite(), starknet_plugin_suite()]
            .into_iter()
            .fold(PluginSuite::default(), |mut acc, suite| {
                acc.add(suite);
                acc
            });
        db_builder.with_plugin_suite(plugin_suite);
        let db = db_builder.build()?;
        // let mut db = Self {
        //     storage: Default::default(),
        // };

        // init_files_group(&mut db);
        // let diag = ScarbDocDatabase::setup_diagnostics();

        // db.set_cfg_set(Self::initial_cfg_set().into());

        // db.apply_plugin_suite(plugin_suite);

        // if let Some(config) = project_config {
        //     db.apply_project_config(config);
        // }

        Ok(Self { db })
    }

    fn initial_cfg_set() -> CfgSet {
        CfgSet::from_iter([Cfg::name("doc")])
    }

    // fn apply_plugin_suite(&mut self, plugin_suite: PluginSuite) {
    //     self.set_macro_plugins(plugin_suite.plugins);
    //     self.set_inline_macro_plugins(plugin_suite.inline_macro_plugins.into());
    //     self.set_analyzer_plugins(plugin_suite.analyzer_plugins);
    // }

    // fn apply_project_config(&mut self, config: ProjectConfig) {
    //     update_crate_roots_from_project_config(self, &config);
    //     if let Some(corelib) = &config.corelib {
    //         update_crate_root(self, &config, CORELIB_CRATE_NAME.into(), corelib.clone());
    //     }
    // }
}

// impl salsa::Database for ScarbDocDatabase {}

// impl ExternalFiles for ScarbDocDatabase {
//     fn ext_as_virtual(&self, external_id: salsa::InternId) -> VirtualFile {
//         ext_as_virtual_impl(self.upcast(), external_id)
//     }
// }

// impl salsa::ParallelDatabase for ScarbDocDatabase {
//     fn snapshot(&self) -> salsa::Snapshot<Self> {
//         salsa::Snapshot::new(ScarbDocDatabase {
//             storage: self.storage.snapshot(),
//         })
//     }
// }

// impl AsFilesGroupMut for ScarbDocDatabase {
//     fn as_files_group_mut(&mut self) -> &mut (dyn FilesGroup + 'static) {
//         self
//     }
// }

// impl Upcast<dyn FilesGroup> for ScarbDocDatabase {
//     fn upcast(&self) -> &(dyn FilesGroup + 'static) {
//         self
//     }
// }

// impl Upcast<dyn ParserGroup> for ScarbDocDatabase {
//     fn upcast(&self) -> &(dyn ParserGroup + 'static) {
//         self
//     }
// }

// impl Upcast<dyn SyntaxGroup> for ScarbDocDatabase {
//     fn upcast(&self) -> &(dyn SyntaxGroup + 'static) {
//         self
//     }
// }

// impl Upcast<dyn DefsGroup> for ScarbDocDatabase {
//     fn upcast(&self) -> &(dyn DefsGroup + 'static) {
//         self
//     }
// }

// impl Upcast<dyn SemanticGroup> for ScarbDocDatabase {
//     fn upcast(&self) -> &(dyn SemanticGroup + 'static) {
//         self
//     }
// }

// impl Upcast<dyn DocGroup> for ScarbDocDatabase {
//     fn upcast(&self) -> &(dyn DocGroup + 'static) {
//         self
//     }
// }
