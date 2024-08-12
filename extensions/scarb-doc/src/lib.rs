use cairo_lang_compiler::project::ProjectConfig;
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::CrateLongId;
use scarb_metadata::{Metadata, PackageMetadata};
use serde::Serialize;

use crate::db::ScarbDocDatabase;
use crate::metadata::compilation::get_project_config;
use types::Crate;

pub mod db;
pub mod docs_generation;
pub mod metadata;
pub mod types;
pub mod versioned_json_output;

#[derive(Serialize)]
pub struct PackageInformation {
    pub crate_: Crate,
    pub metadata: AdditionalMetadata,
}

#[derive(Serialize)]
pub struct AdditionalMetadata {
    pub name: String,
    pub authors: Option<Vec<String>>,
}

pub fn generate_packages_information(
    metadata: &Metadata,
    metadata_for_packages: &[PackageMetadata],
) -> Result<Vec<PackageInformation>, anyhow::Error> {
    let mut packages_information = vec![];
    for package_metadata in metadata_for_packages {
        let authors = package_metadata.manifest_metadata.authors.clone();

        let project_config = get_project_config(metadata, package_metadata);

        let crate_ = generate_language_elements_tree_for_package(
            package_metadata.name.clone(),
            project_config,
        );

        packages_information.push(PackageInformation {
            crate_,
            metadata: AdditionalMetadata {
                name: package_metadata.name.clone(),
                authors,
            },
        });
    }
    Ok(packages_information)
}

fn generate_language_elements_tree_for_package(
    package_name: String,
    project_config: ProjectConfig,
) -> Crate {
    let db = ScarbDocDatabase::new(Some(project_config));

    let main_crate_id = db.intern_crate(CrateLongId::Real(package_name.into()));

    Crate::new(&db, main_crate_id)
}
