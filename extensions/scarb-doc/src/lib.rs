use anyhow::Result;

use cairo_lang_compiler::project::ProjectConfig;
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::CrateLongId;

use crate::db::ScarbDocDatabase;
use types::Crate;

pub mod compilation;
pub mod db;
pub mod types;

pub fn generate_language_elements_tree_for_package(
    package_name: String,
    project_config: ProjectConfig,
) -> Result<Crate> {
    let db = ScarbDocDatabase::new(Some(project_config));

    let main_crate_id = db.intern_crate(CrateLongId::Real(package_name.into()));

    Ok(Crate::new(&db, main_crate_id))
}
