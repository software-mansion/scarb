use scarb_metadata::{
    CompilationUnitComponentMetadata, CompilationUnitMetadata, Metadata, PackageId, PackageMetadata,
};
use smol_str::{SmolStr, ToSmolStr};
use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{bail, Error, Result};
use cairo_lang_compiler::project::{AllCratesConfig, ProjectConfig, ProjectConfigContent};
use cairo_lang_filesystem::cfg::{Cfg, CfgSet};
use cairo_lang_filesystem::db::{
    CrateSettings, DependencySettings, Edition, ExperimentalFeaturesConfig,
};
use cairo_lang_filesystem::ids::Directory;
use cairo_lang_utils::ordered_hash_map::OrderedHashMap;
use itertools::Itertools;

use crate::errors::{
    CfgParseError, MissingCompilationUnitForPackage, MissingCorelibError, MissingPackageError,
};

const LIB_TARGET_KIND: &str = "lib";
const STARKNET_TARGET_KIND: &str = "starknet-contract";
const CORELIB_CRATE_NAME: &str = "core";

pub fn get_project_config(
    metadata: &Metadata,
    package_metadata: &PackageMetadata,
) -> Result<ProjectConfig> {
    let compilation_unit_metadata =
        package_compilation_unit(metadata, package_metadata.id.clone())?;
    let corelib = get_corelib(compilation_unit_metadata)?;
    let dependencies = get_dependencies(compilation_unit_metadata);
    let crates_config = get_crates_config(metadata, compilation_unit_metadata)?;
    Ok(ProjectConfig {
        base_path: package_metadata.root.clone().into(),
        corelib: Some(Directory::Real(corelib.source_root().into())),
        content: ProjectConfigContent {
            crate_roots: dependencies,
            crates_config,
        },
    })
}

fn package_compilation_unit(
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

fn get_corelib(
    compilation_unit_metadata: &CompilationUnitMetadata,
) -> Result<&CompilationUnitComponentMetadata> {
    compilation_unit_metadata
        .components
        .iter()
        .find(|du| du.name == CORELIB_CRATE_NAME)
        .ok_or(Error::new(MissingCorelibError))
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
) -> Result<AllCratesConfig> {
    let crates_config: OrderedHashMap<SmolStr, CrateSettings> = compilation_unit_metadata
        .components
        .iter()
        .map(|component| {
            let pkg = metadata.get_package(&component.package);
            let cfg_result = component
                .cfg
                .as_ref()
                .map(|cfg_vec| build_cfg_set(cfg_vec))
                .transpose();

            match (pkg, cfg_result) {
                (Some(pkg), Ok(cfg_set)) => Ok((
                    SmolStr::from(&component.name),
                    get_crate_settings_for_package(
                        &metadata.packages,
                        &compilation_unit_metadata.components,
                        pkg,
                        cfg_set,
                    )?,
                )),
                (None, _) => {
                    bail!(MissingPackageError(component.package.to_string()))
                }
                (_, Err(e)) => bail!(e),
            }
        })
        .collect::<Result<OrderedHashMap<SmolStr, CrateSettings>>>()?;

    Ok(AllCratesConfig {
        override_map: crates_config,
        ..Default::default()
    })
}

fn get_crate_settings_for_package(
    packages: &[PackageMetadata],
    compilation_unit_metadata_components: &[CompilationUnitComponentMetadata],
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

    let mut dependencies: BTreeMap<String, DependencySettings> = package
        .dependencies
        .iter()
        .filter_map(|dependency| {
            compilation_unit_metadata_components
                .iter()
                .find(|compilation_unit_metadata_component| {
                    compilation_unit_metadata_component.name == dependency.name
                })
                .map(|compilation_unit_metadata_component| {
                    let version = packages
                        .iter()
                        .find(|package| package.name == compilation_unit_metadata_component.name)
                        .map(|package| package.version.clone());
                    let version = (dependency.name == *CORELIB_CRATE_NAME)
                        .then_some(version)
                        .flatten();
                    (dependency.name.clone(), DependencySettings { version })
                })
        })
        .collect();

    // Adds itself to dependencies
    dependencies.insert(
        package.name.clone(),
        DependencySettings {
            version: (package.name != *CORELIB_CRATE_NAME).then_some(package.version.clone()),
        },
    );

    Ok(CrateSettings {
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
