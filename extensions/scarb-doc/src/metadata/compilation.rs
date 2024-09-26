use scarb_metadata::{
    CompilationUnitComponentMetadata, CompilationUnitMetadata, Metadata, PackageId, PackageMetadata,
};
use smol_str::{SmolStr, ToSmolStr};
use std::path::PathBuf;

use cairo_lang_compiler::project::{AllCratesConfig, ProjectConfig, ProjectConfigContent};
use cairo_lang_filesystem::cfg::{Cfg, CfgSet};
use cairo_lang_filesystem::db::{
    CrateSettings, DependencySettings, Edition, ExperimentalFeaturesConfig,
};
use cairo_lang_filesystem::ids::Directory;
use cairo_lang_utils::ordered_hash_map::OrderedHashMap;
use itertools::Itertools;

const LIB_TARGET_KIND: &str = "lib";
const STARKNET_TARGET_KIND: &str = "starknet-contract";
const CORELIB_CRATE_NAME: &str = "core";

pub fn get_project_config(
    metadata: &Metadata,
    package_metadata: &PackageMetadata,
) -> ProjectConfig {
    let compilation_unit_metadata = package_compilation_unit(metadata, package_metadata.id.clone());
    let corelib = get_corelib(compilation_unit_metadata);
    let dependencies = get_dependencies(compilation_unit_metadata);
    let crates_config = get_crates_config(metadata, compilation_unit_metadata);
    ProjectConfig {
        base_path: package_metadata.root.clone().into(),
        corelib: Some(Directory::Real(corelib.source_root().into())),
        content: ProjectConfigContent {
            crate_roots: dependencies,
            crates_config,
        },
    }
}

fn package_compilation_unit(
    metadata: &Metadata,
    package_id: PackageId,
) -> &CompilationUnitMetadata {
    let relevant_cus = metadata
        .compilation_units
        .iter()
        .filter(|m| m.package == package_id)
        .collect_vec();

    relevant_cus
        .iter()
        .find(|m| m.target.kind == LIB_TARGET_KIND)
        .or_else(|| {
            relevant_cus
                .iter()
                .find(|m| m.target.kind == STARKNET_TARGET_KIND)
        })
        .expect("failed to find compilation unit for package")
}

fn get_corelib(
    compilation_unit_metadata: &CompilationUnitMetadata,
) -> &CompilationUnitComponentMetadata {
    compilation_unit_metadata
        .components
        .iter()
        .find(|du| du.name == CORELIB_CRATE_NAME)
        .expect("Corelib could not be found")
}

fn get_dependencies(
    compilation_unit_metadata: &CompilationUnitMetadata,
) -> OrderedHashMap<SmolStr, PathBuf> {
    compilation_unit_metadata
        .components
        .iter()
        .filter(|du| du.name != CORELIB_CRATE_NAME)
        .map(|cu| {
            (
                cu.name.to_smolstr(),
                cu.source_root().to_owned().into_std_path_buf(),
            )
        })
        .collect()
}

fn get_crates_config(
    metadata: &Metadata,
    compilation_unit_metadata: &CompilationUnitMetadata,
) -> AllCratesConfig {
    let crates_config: OrderedHashMap<SmolStr, CrateSettings> = compilation_unit_metadata
        .components
        .iter()
        .map(|component| {
            let pkg = metadata.get_package(&component.package).unwrap_or_else(|| {
                panic!(
                    "failed to find = {} package",
                    &component.package.to_string()
                )
            });
            (
                SmolStr::from(&component.name),
                get_crate_settings_for_package(
                    pkg,
                    component.cfg.as_ref().map(|cfg_vec| build_cfg_set(cfg_vec)),
                ),
            )
        })
        .collect();

    AllCratesConfig {
        override_map: crates_config,
        ..Default::default()
    }
}

fn get_crate_settings_for_package(
    package: &PackageMetadata,
    cfg_set: Option<CfgSet>,
) -> CrateSettings {
    let edition = package
        .edition
        .clone()
        .map_or(Edition::default(), |edition| {
            let edition_value = serde_json::Value::String(edition);
            serde_json::from_value(edition_value).unwrap()
        });

    let experimental_features = ExperimentalFeaturesConfig {
        negative_impls: package
            .experimental_features
            .contains(&String::from("negative_impls")),
        coupons: package
            .experimental_features
            .contains(&String::from("coupons")),
    };

    let dependencies = package
        .dependencies
        .iter()
        .map(|d| (d.name.clone(), DependencySettings { version: None }))
        .collect();

    CrateSettings {
        edition,
        cfg_set,
        experimental_features,
        dependencies,
        version: Some(package.version.clone()),
    }
}

fn build_cfg_set(cfg: &[scarb_metadata::Cfg]) -> CfgSet {
    CfgSet::from_iter(cfg.iter().map(|cfg| {
        serde_json::to_value(cfg)
            .and_then(serde_json::from_value::<Cfg>)
            .expect("Cairo's `Cfg` must serialize identically as Scarb Metadata's `Cfg`.")
    }))
}
