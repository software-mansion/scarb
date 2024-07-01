use anyhow::Result;

use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::project::ProjectConfig;
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::CrateLongId;
use cairo_lang_starknet::starknet_plugin_suite;

use types::Crate;

pub mod compilation;
mod types;

pub fn generate_language_elements_tree_for_package(
    package_name: String,
    project_config: ProjectConfig,
) -> Result<Crate> {
    let db = &mut {
        let mut b = RootDatabase::builder();
        b.with_project_config(project_config);
        b.with_plugin_suite(starknet_plugin_suite());
        b.build()?
    };

    let main_crate_id = db.intern_crate(CrateLongId::Real(package_name.into()));

    Ok(Crate::new(db, main_crate_id))
}
