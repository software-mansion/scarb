use cairo_lang_compiler::project::ProjectConfig;
use cairo_lang_filesystem::cfg::{Cfg, CfgSet};
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::CrateLongId;
use scarb::core::TomlManifest;
use scarb_metadata::{Metadata, PackageMetadata};
use serde::Serialize;
use std::collections::BTreeMap;

use crate::db::ScarbDocDatabase;
use crate::metadata::compilation::get_project_config;
use scarb::core::FeatureName;
use scarb::ops::{get_cfg_with_features, FeaturesOpts};
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
    enabled_features: FeaturesOpts,
) -> Result<Vec<PackageInformation>, anyhow::Error> {
    let mut packages_information = vec![];
    for package_metadata in metadata_for_packages {
        let authors = package_metadata.manifest_metadata.authors.clone();

        let features_manifest: BTreeMap<FeatureName, Vec<FeatureName>> =
            match TomlManifest::read_from_path(&package_metadata.manifest_path)?.features {
                Some(features_manifest) => features_manifest,
                None => BTreeMap::new(),
            };

        let initial_cfg_set = initial_cfg_set();
        let cfg_wtih_features = get_cfg_with_features(
            initial_cfg_set.clone(),
            &features_manifest,
            &enabled_features,
            false,
        )?;
        let cfg = match cfg_wtih_features {
            Some(cfg_with_features) => cfg_with_features,
            None => initial_cfg_set,
        };

        let project_config = get_project_config(metadata, package_metadata);

        let crate_ = generate_language_elements_tree_for_package(
            package_metadata.name.clone(),
            project_config.clone(),
            cfg.clone(),
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
    features_cfg: CfgSet,
) -> Crate {
    let db: ScarbDocDatabase =
        ScarbDocDatabase::new(&package_name, Some(project_config), features_cfg);

    let main_crate_id = db.intern_crate(CrateLongId::Real(package_name.into()));

    Crate::new(&db, main_crate_id)
}

fn initial_cfg_set() -> CfgSet {
    CfgSet::from_iter([Cfg::name("doc")])
}
