use std::collections::BTreeMap;

use semver::VersionReq;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use toml::Value;

pub use compiler_config::*;
pub use dependency::*;
pub use scripts::*;
pub use summary::*;
pub use target::*;
pub use toml_manifest::*;

use crate::compiler::Profile;

mod compiler_config;
mod dependency;
mod scripts;
mod summary;
mod target;
mod toml_manifest;

/// Contains all the information about a package, as loaded from the manifest file.
///
/// This is deserialized using the [`TomlManifest`] type.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Manifest {
    pub summary: Summary,
    pub targets: Vec<Target>,
    pub metadata: ManifestMetadata,
    pub compiler_config: ManifestCompilerConfig,
    pub scripts: BTreeMap<SmolStr, ScriptDefinition>,
    pub profiles: Vec<Profile>,
}

/// Subset of a [`Manifest`] that contains package metadata.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ManifestMetadata {
    pub authors: Option<Vec<String>>,
    pub urls: Option<BTreeMap<String, String>>,
    pub description: Option<String>,
    pub documentation: Option<String>,
    pub homepage: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub license: Option<String>,
    pub license_file: Option<String>,
    pub readme: Option<String>,
    pub repository: Option<String>,
    #[serde(rename = "tool")]
    pub tool_metadata: Option<BTreeMap<SmolStr, Value>>,
    pub cairo_version: Option<VersionReq>,
}
