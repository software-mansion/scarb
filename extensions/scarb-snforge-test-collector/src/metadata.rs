use anyhow::{anyhow, ensure, Context, Result};
use cairo_lang_filesystem::cfg::{Cfg, CfgSet};
use cairo_lang_filesystem::db::{
    CrateSettings, DependencySettings, Edition, ExperimentalFeaturesConfig,
};
use cairo_lang_project::AllCratesConfig;
use cairo_lang_utils::ordered_hash_map::OrderedHashMap;
use camino::{Utf8Path, Utf8PathBuf};
use itertools::Itertools;
use scarb_metadata::{
    CompilationUnitComponentMetadata, CompilationUnitMetadata, Metadata, PackageMetadata,
};
use serde_json::json;
use smol_str::{SmolStr, ToSmolStr};
use std::path::PathBuf;

pub fn compilation_unit_for_package<'a>(
    metadata: &'a Metadata,
    package_metadata: &PackageMetadata,
) -> Result<CompilationUnit<'a>> {
    let unit_test_cu = metadata
        .compilation_units
        .iter()
        .find(|unit| {
            unit.package == package_metadata.id
                && unit.target.kind == "test"
                && unit.target.params.get("test-type") == Some(&json!("unit"))
        })
        .ok_or_else(|| {
            anyhow!(
                "Failed to find unit test compilation unit for package = {}",
                package_metadata.name
            )
        })?;
    let all_test_cus = metadata
        .compilation_units
        .iter()
        .filter(|unit| unit.package == package_metadata.id && unit.target.kind == "test")
        .collect_vec();

    let unit_test_deps = unit_test_cu.components.iter().collect_vec();

    for cu in all_test_cus {
        let test_type = cu
            .target
            .params
            .get("test-type")
            .expect("Test target missing test-type param")
            .as_str()
            .expect("test-type param is not a string");

        let test_deps_without_tests = cu
            .components
            .iter()
            .filter(|du| match test_type {
                "unit" => true,
                _ => !du.source_root().starts_with(cu.target.source_root()),
            })
            .collect_vec();

        ensure!(
            unit_test_deps == test_deps_without_tests,
            "Dependencies mismatch between test compilation units"
        );
    }

    let main_package_metadata = unit_test_cu
        .components
        .iter()
        .find(|comp| comp.package == package_metadata.id)
        .into_iter()
        .collect_vec();

    assert_eq!(
        main_package_metadata.len(),
        1,
        "More than one cu component with main package id found"
    );

    Ok(CompilationUnit {
        unit_metadata: unit_test_cu,
        main_package_metadata: main_package_metadata[0],
        metadata,
    })
}

pub struct CompilationUnit<'a> {
    unit_metadata: &'a CompilationUnitMetadata,
    main_package_metadata: &'a CompilationUnitComponentMetadata,
    metadata: &'a Metadata,
}

impl CompilationUnit<'_> {
    pub fn dependencies(&self) -> OrderedHashMap<SmolStr, PathBuf> {
        let dependencies = self
            .unit_metadata
            .components
            .iter()
            .filter(|du| &du.name != "core")
            .map(|cu| {
                (
                    cu.name.to_smolstr(),
                    cu.source_root().to_owned().into_std_path_buf(),
                )
            })
            .collect();

        dependencies
    }

    pub fn corelib_path(&self) -> Result<PathBuf> {
        let corelib = self
            .unit_metadata
            .components
            .iter()
            .find(|du| du.name == "core")
            .context("Corelib could not be found")?;
        Ok(PathBuf::from(corelib.source_root()))
    }

    pub fn crates_config_for_compilation_unit(&self) -> AllCratesConfig {
        let crates_config: OrderedHashMap<SmolStr, CrateSettings> = self
            .unit_metadata
            .components
            .iter()
            .map(|component| {
                let pkg = self
                    .metadata
                    .get_package(&component.package)
                    .unwrap_or_else(|| {
                        panic!(
                            "Failed to find = {} package",
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

    /// Retrieve `allow-warnings` flag from the compiler config.
    pub fn allow_warnings(&self) -> bool {
        self.unit_metadata
            .compiler_config
            .as_object()
            .and_then(|config| config.get("allow_warnings"))
            .and_then(|value| value.as_bool())
            .unwrap_or(true)
    }

    pub fn unstable_add_statements_functions_debug_info(&self) -> bool {
        self.unit_metadata
            .compiler_config
            .as_object()
            .and_then(|config| config.get("unstable_add_statements_functions_debug_info"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    }

    pub fn unstable_add_statements_code_locations_debug_info(&self) -> bool {
        self.unit_metadata
            .compiler_config
            .as_object()
            .and_then(|config| config.get("unstable_add_statements_code_locations_debug_info"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    }

    pub fn main_package_source_root(&self) -> Utf8PathBuf {
        self.main_package_metadata.source_root().to_path_buf()
    }

    pub fn main_package_source_file_path(&self) -> &Utf8Path {
        &self.main_package_metadata.source_path
    }

    pub fn main_package_crate_settings(&self) -> CrateSettings {
        let package = self
            .metadata
            .packages
            .iter()
            .find(|package| package.id == self.main_package_metadata.package)
            .expect("Main package not found in metadata");

        get_crate_settings_for_package(
            package,
            self.main_package_metadata
                .cfg
                .as_ref()
                .map(|cfg_vec| build_cfg_set(cfg_vec)),
        )
    }

    pub fn compilation_unit_cfg_set(&self) -> CfgSet {
        build_cfg_set(&self.unit_metadata.cfg)
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
    // TODO (#1040): replace this with a macro
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
