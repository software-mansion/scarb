use anyhow::{anyhow, Context, Result};
use cairo_lang_filesystem::db::{CrateSettings, Edition};
use cairo_lang_project::AllCratesConfig;
use cairo_lang_utils::ordered_hash_map::OrderedHashMap;
use camino::{Utf8Path, Utf8PathBuf};
use scarb_metadata::{CompilationUnitMetadata, Metadata, PackageMetadata};
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
    let compilation_unit_metadata = metadata
        .compilation_units
        .iter()
        .filter(|unit| unit.package == package_metadata.id)
        .min_by_key(|unit| match unit.target.kind.as_str() {
            name @ "starknet-contract" => (0, name),
            name @ "lib" => (1, name),
            name => (2, name),
        })
        .ok_or_else(|| {
            anyhow!(
                "Failed to find compilation unit for package = {}",
                package_metadata.name
            )
        })?;
    Ok(CompilationUnit {
        unit_metadata: compilation_unit_metadata,
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
                (
                    SmolStr::from(&component.name),
                    CrateSettings {
                        edition: if let Some(edition) = self
                            .metadata
                            .get_package(&component.package)
                            .unwrap_or_else(|| {
                                panic!("Failed to find = {} package", component.package)
                            })
                            .edition
                            .clone()
                        {
                            let edition_value = serde_json::Value::String(edition);
                            serde_json::from_value(edition_value).unwrap()
                        } else {
                            Edition::default()
                        },
                        experimental_features: Default::default(),
                    },
                )
            })
            .collect();

        AllCratesConfig {
            override_map: crates_config,
            ..Default::default()
        }
    }

    pub fn source_root(&self) -> Utf8PathBuf {
        self.unit_metadata.target.source_root().to_path_buf()
    }

    pub fn source_file_path(&self) -> &Utf8Path {
        &self.unit_metadata.target.source_path
    }
}
