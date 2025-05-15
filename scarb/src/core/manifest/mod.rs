use anyhow::{Result, bail, ensure};
use cairo_lang_filesystem::db::Edition;
use camino::Utf8PathBuf;
use derive_builder::Builder;
use itertools::Itertools;
use semver::VersionReq;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::collections::btree_map::Keys;
use std::collections::{BTreeMap, HashSet};
use toml::Value;

pub use compiler_config::*;
pub use dependency::*;
pub use maybe_workspace::*;
pub use scripts::*;
pub use summary::*;
pub use target::*;
pub use target_kind::*;
pub use toml_manifest::*;
pub use version_req::*;

use crate::compiler::DefaultForProfile;
use crate::compiler::Profile;

use super::PackageName;

mod compiler_config;
mod dependency;
mod maybe_workspace;
mod scripts;
mod summary;
mod target;
mod target_kind;
mod toml_manifest;
mod version_req;

pub type FeatureName = PackageName;
pub const DEFAULT_FEATURE_NAME: &str = "default";

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
    pub edition: Edition,
    #[builder(default = "true")]
    pub publish: bool,
    #[builder(default)]
    pub metadata: ManifestMetadata,
    #[builder(default = "ManifestCompilerConfig::default_for_profile(&Profile::DEV)")]
    pub compiler_config: ManifestCompilerConfig,
    #[builder(default)]
    pub scripts: BTreeMap<SmolStr, ScriptDefinition>,
    #[builder(default)]
    pub features: FeaturesDefinition,
    /// Allow experimental features.
    #[builder(default)]
    pub experimental_features: Option<Vec<SmolStr>>,
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
    pub license_file: Option<Utf8PathBuf>,
    pub readme: Option<Utf8PathBuf>,
    pub repository: Option<String>,
    pub include: Option<Vec<Utf8PathBuf>>,
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
        let Some(targets) = &self.targets else {
            return Ok(());
        };

        if targets.iter().any(Target::is_cairo_plugin) {
            ensure!(
                targets.len() == 1,
                "target `{}` cannot be mixed with other targets",
                TargetKind::CAIRO_PLUGIN,
            );
        }
        Ok(())
    }

    fn check_unique_targets(&self) -> Result<()> {
        let Some(summary) = &self.summary else {
            return Ok(());
        };
        let Some(targets) = &self.targets else {
            return Ok(());
        };

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

pub fn edition_variant(edition: Edition) -> String {
    let edition = serde_json::to_value(edition).unwrap();
    let serde_json::Value::String(edition) = edition else {
        panic!("Edition should always be a string.")
    };
    edition
}

#[derive(Clone, Debug)]
pub struct EnabledFeature {
    pub package: Option<PackageName>,
    pub feature: FeatureName,
}

#[derive(Clone, Debug, Default)]
pub struct FeaturesDefinition(BTreeMap<FeatureName, Vec<EnabledFeature>>);

impl FeaturesDefinition {
    pub fn try_new(features: BTreeMap<FeatureName, Vec<EnabledFeature>>) -> Result<Self> {
        Self::validate(&features)?;
        Ok(Self(features))
    }

    fn validate(features: &BTreeMap<FeatureName, Vec<EnabledFeature>>) -> Result<()> {
        let available_features: HashSet<&FeatureName> = features.keys().collect();
        for (key, vals) in features.iter() {
            let dependent_features = vals
                .iter()
                // Skip dependency features, as they need to be validated with dependency manifest.
                .filter(|f| f.package.is_none())
                .map(|f| &f.feature)
                .collect::<HashSet<&FeatureName>>();
            ensure!(
                !dependent_features.contains(key),
                "feature `{}` depends on itself",
                key
            );
            let not_found_features = dependent_features
                .difference(&available_features)
                .collect_vec();
            ensure!(
                not_found_features.is_empty(),
                "feature `{}` is dependent on `{}` which is not defined",
                key,
                not_found_features.iter().join(", "),
            );
        }
        Ok(())
    }

    pub fn all(&self) -> Keys<'_, FeatureName, Vec<EnabledFeature>> {
        self.0.keys()
    }

    pub fn get(&self, feature: &FeatureName) -> Option<&Vec<EnabledFeature>> {
        self.0.get(feature)
    }

    pub fn contains_key(&self, feature: &FeatureName) -> bool {
        self.0.contains_key(feature)
    }

    pub fn default_features(&self) -> Vec<EnabledFeature> {
        self.0
            .get(DEFAULT_FEATURE_NAME)
            .cloned()
            .unwrap_or_default()
    }

    /// Return list of features enabled in this package via user args.
    /// Note: This does not resolve dependant features! Only user input will be returned.
    pub fn select(
        &self,
        enabled_features: &[FeatureName],
        default_enabled: bool,
    ) -> SelectedFeatures {
        let available_features = self.all().cloned().collect::<HashSet<FeatureName>>();
        let mut selected_features: HashSet<FeatureName> =
            enabled_features.iter().cloned().collect();
        if default_enabled {
            let default_features: Vec<FeatureName> = self
                .default_features()
                .into_iter()
                // We filter only features enabled in this package, because we use this list to
                // find dependant features. Features enabled for dependencies will be collected
                // separately during dependant features resolution, for all enabled top-level features.
                .filter(|f| f.package.is_none())
                .map(|f| f.feature)
                .collect();
            selected_features.extend(default_features);
            selected_features.insert(unsafe { FeatureName::new_unchecked(DEFAULT_FEATURE_NAME) });
        }
        let mut not_found_features: HashSet<FeatureName> = selected_features
            .difference(&available_features)
            .cloned()
            .collect();
        not_found_features.remove(DEFAULT_FEATURE_NAME);
        let enabled = available_features
            .intersection(&selected_features)
            .cloned()
            .collect();
        SelectedFeatures::new(enabled, not_found_features)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&FeatureName, &Vec<EnabledFeature>)> {
        self.0.iter()
    }
}

impl TryFrom<BTreeMap<FeatureName, Vec<TomlFeatureToEnable>>> for FeaturesDefinition {
    type Error = anyhow::Error;
    fn try_from(features: BTreeMap<FeatureName, Vec<TomlFeatureToEnable>>) -> Result<Self> {
        Self::try_new(
            features
                .into_iter()
                .map(|(name, enabled)| {
                    Ok((
                        name,
                        enabled
                            .into_iter()
                            .map(TryFrom::try_from)
                            .collect::<Result<_>>()?,
                    ))
                })
                .collect::<Result<_>>()?,
        )
    }
}

pub struct SelectedFeatures {
    enabled: HashSet<FeatureName>,
    not_found: HashSet<FeatureName>,
}

impl SelectedFeatures {
    fn new(enabled: HashSet<FeatureName>, not_found: HashSet<FeatureName>) -> Self {
        Self { enabled, not_found }
    }

    pub fn enabled(self) -> HashSet<FeatureName> {
        self.enabled
    }

    pub fn validate(&self) -> Result<()> {
        if !self.not_found.is_empty() {
            bail!("unknown features: {}", self.not_found.iter().join(", "));
        }
        Ok(())
    }
}

impl TryFrom<SelectedFeatures> for HashSet<FeatureName> {
    type Error = anyhow::Error;

    fn try_from(value: SelectedFeatures) -> Result<Self> {
        value.validate()?;
        Ok(value.enabled())
    }
}
