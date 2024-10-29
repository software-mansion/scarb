use scarb_metadata::{
    CompilationUnitComponentDependencyMetadata, CompilationUnitComponentMetadata,
    CompilationUnitMetadata, Metadata, PackageId, PackageMetadata,
};
use smol_str::ToSmolStr;
use std::path::PathBuf;

use anyhow::{bail, Result};
use cairo_lang_compiler::project::{AllCratesConfig, ProjectConfig, ProjectConfigContent};
use cairo_lang_filesystem::cfg::{Cfg, CfgSet};
use cairo_lang_filesystem::db::{
    CrateIdentifier, CrateSettings, DependencySettings, Edition, ExperimentalFeaturesConfig,
};
use cairo_lang_utils::ordered_hash_map::OrderedHashMap;
use itertools::Itertools;

use crate::errors::{CfgParseError, MissingCompilationUnitForPackage, MissingPackageError};

const LIB_TARGET_KIND: &str = "lib";
const STARKNET_TARGET_KIND: &str = "starknet-contract";

pub fn get_project_config(
    metadata: &Metadata,
    package: &PackageMetadata,
    unit: &CompilationUnitMetadata,
) -> Result<ProjectConfig> {
    let crate_roots = get_crate_roots(unit);
    let crates_config = get_crates_config(metadata, unit)?;
    Ok(ProjectConfig {
        base_path: package.root.clone().into(),
        content: ProjectConfigContent {
            crate_roots,
            crates_config,
        },
    })
}

pub fn get_relevant_compilation_unit(
    metadata: &Metadata,
    package_id: PackageId,
) -> Result<&CompilationUnitMetadata, MissingCompilationUnitForPackage> {
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
        .ok_or(MissingCompilationUnitForPackage(package_id.to_string()))
        .copied()
}

fn get_crate_roots(
    compilation_unit_metadata: &CompilationUnitMetadata,
) -> OrderedHashMap<CrateIdentifier, PathBuf> {
    compilation_unit_metadata
        .components
        .iter()
        .map(|cu| {
            (
                cu.id
                    .as_ref()
                    .expect("component is expected to have an id")
                    .into(),
                cu.source_root().to_owned().into_std_path_buf(),
            )
        })
        .collect()
}

fn get_crates_config(
    metadata: &Metadata,
    unit: &CompilationUnitMetadata,
) -> Result<AllCratesConfig> {
    let crates_config = unit
        .components
        .iter()
        .map(|component| {
            let package = metadata.get_package(&component.package);
            let cfg_result = component
                .cfg
                .as_ref()
                .map(|cfg_vec| build_cfg_set(cfg_vec))
                .transpose();

            match (package, cfg_result) {
                (Some(package), Ok(cfg_set)) => Ok((
                    component
                        .id
                        .as_ref()
                        .expect("component is expected to have an id")
                        .into(),
                    get_crate_settings_for_component(component, unit, package, cfg_set)?,
                )),
                (None, _) => {
                    bail!(MissingPackageError(component.package.to_string()))
                }
                (_, Err(e)) => bail!(e),
            }
        })
        .collect::<Result<OrderedHashMap<CrateIdentifier, CrateSettings>>>()?;

    Ok(AllCratesConfig {
        override_map: crates_config,
        ..Default::default()
    })
}

fn get_crate_settings_for_component(
    component: &CompilationUnitComponentMetadata,
    unit: &CompilationUnitMetadata,
    package: &PackageMetadata,
    cfg_set: Option<CfgSet>,
) -> Result<CrateSettings> {
    let edition = package
        .edition
        .clone()
        .map_or(Ok(Edition::default()), |edition| {
            let edition_value = serde_json::Value::String(edition);
            serde_json::from_value(edition_value)
        })?;

    let experimental_features = ExperimentalFeaturesConfig {
        negative_impls: package
            .experimental_features
            .contains(&String::from("negative_impls")),
        coupons: package
            .experimental_features
            .contains(&String::from("coupons")),
    };

    let dependencies = component
        .dependencies
        .as_ref()
        .expect("dependencies are expected to exist")
        .iter()
        .map(|CompilationUnitComponentDependencyMetadata { id, .. }| {
            let dependency_component = unit.components.iter()
                .find(|component| component.id.as_ref().expect("component is expected to have an id") == id)
                .expect("dependency of a component is guaranteed to exist in compilation unit components");
            (
                dependency_component.name.clone(),
                DependencySettings {
                    discriminator: dependency_component.discriminator.as_ref().map(ToSmolStr::to_smolstr)
                },
            )
        })
        .collect();

    Ok(CrateSettings {
        name: Some(component.name.to_smolstr()),
        edition,
        cfg_set,
        experimental_features,
        dependencies,
        version: Some(package.version.clone()),
    })
}

fn build_cfg_set(cfg: &[scarb_metadata::Cfg]) -> Result<CfgSet, CfgParseError> {
    cfg.iter()
        .map(|cfg| {
            serde_json::to_value(cfg)
                .and_then(serde_json::from_value::<Cfg>)
                .map_err(CfgParseError::from)
        })
        .collect::<Result<CfgSet, CfgParseError>>()
}
