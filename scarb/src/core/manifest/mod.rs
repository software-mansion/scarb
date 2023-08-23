use std::collections::{BTreeMap, HashSet};

use anyhow::{bail, ensure, Result};
use derive_builder::Builder;
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
pub use version_req::*;

use crate::compiler::DefaultForProfile;
use crate::compiler::Profile;

mod compiler_config;
mod dependency;
mod maybe_workspace;
mod scripts;
mod summary;
mod target;
mod toml_manifest;
mod version_req;

/// Contains all the information about a package, as loaded from the manifest file.
/// Construct using [`ManifestBuilder`].
/// This is deserialized using the [`TomlManifest`] type.
#[derive(Builder, Clone, Debug)]
#[builder(build_fn(error = "anyhow::Error", validate = "Self::check"))]
#[non_exhaustive]
pub struct Manifest {
    pub summary: Summary,
    pub targets: Vec<Target>,
    #[builder(default)]
    pub metadata: ManifestMetadata,
    #[builder(default = "ManifestCompilerConfig::default_for_profile(&Profile::DEV)")]
    pub compiler_config: ManifestCompilerConfig,
    #[builder(default)]
    pub scripts: BTreeMap<SmolStr, ScriptDefinition>,
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

impl ManifestBuilder {
    fn check(&self) -> Result<()> {
        self.check_cairo_plugin_target_is_exclusive()?;
        self.check_unique_targets()?;
        Ok(())
    }

    fn check_cairo_plugin_target_is_exclusive(&self) -> Result<()> {
        let Some(targets) = &self.targets else { return Ok(()); };

        if targets.iter().any(Target::is_cairo_plugin) {
            ensure!(
                targets.len() == 1,
                "target `{}` cannot be mixed with other targets",
                Target::CAIRO_PLUGIN,
            );
        }
        Ok(())
    }

    fn check_unique_targets(&self) -> Result<()> {
        let Some(summary) = &self.summary else { return Ok(()); };
        let Some(targets) = &self.targets else { return Ok(()); };

        let mut used = HashSet::with_capacity(targets.len());
        for target in targets {
            if !used.insert((target.kind.as_str(), target.name.as_str())) {
                if target.name == summary.package_id.name.as_str() {
                    bail!(
                        "manifest contains duplicate target definitions `{}`, \
                        consider explicitly naming targets with the `name` field",
                        target.kind
                    )
                } else {
                    bail!(
                        "manifest contains duplicate target definitions `{} ({})`, \
                        use different target names to resolve the conflict",
                        target.kind,
                        target.name
                    )
                }
            }
        }
        Ok(())
    }
}
