use anyhow::{anyhow, ensure, Context, Result};
use cairo_lang_filesystem::db::{CrateSettings, Edition, ExperimentalFeaturesConfig};
use cairo_lang_project::AllCratesConfig;
use cairo_lang_utils::ordered_hash_map::OrderedHashMap;
use camino::{Utf8Path, Utf8PathBuf};
use itertools::Itertools;
use scarb_metadata::{CompilationUnitMetadata, Metadata, PackageMetadata};
use serde_json::json;
use smol_str::SmolStr;
use std::path::PathBuf;

/// Represents a dependency of a Cairo project
#[derive(Debug, Clone)]
pub struct LinkedLibrary {
    pub name: String,
    pub path: PathBuf,
}

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

    Ok(CompilationUnit {
        unit_metadata: unit_test_cu,
        metadata,
    })
}

pub struct CompilationUnit<'a> {
    unit_metadata: &'a CompilationUnitMetadata,
    metadata: &'a Metadata,
}

impl CompilationUnit<'_> {
    pub fn dependencies(&self) -> Vec<LinkedLibrary> {
        let dependencies = self
            .unit_metadata
            .components
            .iter()
            .filter(|du| &du.name != "core")
            .map(|cu| LinkedLibrary {
                name: cu.name.clone(),
                path: cu.source_root().to_owned().into_std_path_buf(),
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
                    .unwrap_or_else(|| panic!("Failed to find = {} package", &component.package));
                (
                    SmolStr::from(&component.name),
                    CrateSettings {
                        edition: if let Some(edition) = pkg.edition.clone() {
                            let edition_value = serde_json::Value::String(edition);
                            serde_json::from_value(edition_value).unwrap()
                        } else {
                            Edition::default()
                        },
                        // TODO (#1040): replace this with a macro
                        experimental_features: ExperimentalFeaturesConfig {
                            negative_impls: pkg
                                .allow_features
                                .contains(&String::from("negative_impls")),
                        },
                    },
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

    pub fn source_root(&self) -> Utf8PathBuf {
        self.unit_metadata.target.source_root().to_path_buf()
    }

    pub fn source_file_path(&self) -> &Utf8Path {
        &self.unit_metadata.target.source_path
    }
}
