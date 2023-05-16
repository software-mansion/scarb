use std::collections::{BTreeMap, HashSet};
use std::default::Default;
use std::fs;

use crate::compiler::{DefaultForProfile, Profile};
use anyhow::{bail, ensure, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use itertools::Itertools;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::str::FromStr;
use toml::Value;
use tracing::trace;
use url::Url;

use crate::core::manifest::scripts::ScriptDefinition;
use crate::core::manifest::{ManifestDependency, ManifestMetadata, Summary, Target};
use crate::core::package::PackageId;
use crate::core::source::{GitReference, SourceId};
use crate::core::{ManifestCompilerConfig, PackageName};
use crate::internal::fsx;
use crate::internal::fsx::PathUtf8Ext;
use crate::internal::to_version::ToVersion;
use crate::DEFAULT_SOURCE_PATH;

use super::Manifest;

/// This type is used to deserialize `Scarb.toml` files.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TomlManifest {
    pub package: Option<Box<TomlPackage>>,
    pub dependencies: Option<BTreeMap<PackageName, TomlDependency>>,
    pub lib: Option<TomlTarget<TomlLibTargetParams>>,
    pub target: Option<BTreeMap<TomlTargetKind, Vec<TomlTarget<TomlExternalTargetParams>>>>,
    pub cairo: Option<TomlCairo>,
    pub tool: Option<ToolDefinition>,
    pub scripts: Option<BTreeMap<SmolStr, String>>,
    pub profile: Option<BTreeMap<SmolStr, TomlProfile>>,
}

type ToolDefinition = BTreeMap<SmolStr, Value>;

/// Represents the `package` section of a `Scarb.toml`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TomlPackage {
    pub name: PackageName,
    pub version: Version,
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
    /// **UNSTABLE** This package does not depend on Cairo's `core`.
    pub no_core: Option<bool>,
    pub cairo_version: Option<VersionReq>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TomlDependency {
    /// [`VersionReq`] specified as a string, eg. `package = "<version>"`.
    Simple(VersionReq),
    /// Detailed specification as a table, eg. `package = { version = "<version>" }`.
    Detailed(DetailedTomlDependency),
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DetailedTomlDependency {
    pub version: Option<VersionReq>,

    /// Relative to the file it appears in.
    pub path: Option<Utf8PathBuf>,

    pub git: Option<Url>,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub rev: Option<String>,
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(into = "SmolStr", try_from = "SmolStr")]
pub struct TomlTargetKind(SmolStr);

impl TomlTargetKind {
    pub fn try_new(name: SmolStr) -> Result<Self> {
        ensure!(name != Target::LIB, "target kind `{name}` is reserved");
        Ok(Self(name))
    }

    pub fn to_smol_str(&self) -> SmolStr {
        self.0.clone()
    }

    pub fn into_smol_str(self) -> SmolStr {
        self.0
    }
}

impl From<TomlTargetKind> for SmolStr {
    fn from(value: TomlTargetKind) -> Self {
        value.into_smol_str()
    }
}

impl TryFrom<SmolStr> for TomlTargetKind {
    type Error = anyhow::Error;

    fn try_from(value: SmolStr) -> Result<Self> {
        TomlTargetKind::try_new(value)
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TomlTarget<P> {
    pub name: Option<SmolStr>,

    #[serde(flatten)]
    pub params: P,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TomlLibTargetParams {
    pub sierra: Option<bool>,
    pub casm: Option<bool>,
}

pub type TomlExternalTargetParams = BTreeMap<SmolStr, Value>;

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct TomlCairo {
    /// Replace all names in generated Sierra code with dummy counterparts, representing the
    /// expanded information about the named items.
    ///
    /// For libfuncs and types that would be recursively opening their generic arguments.
    /// For functions, that would be their original name in Cairo.
    /// For example, while the Sierra name be `[6]`, with this flag turned on it might be:
    /// - For libfuncs: `felt252_const<2>` or `unbox<Box<Box<felt252>>>`.
    /// - For types: `felt252` or `Box<Box<felt252>>`.
    /// - For user functions: `test::foo`.
    ///
    /// Defaults to `false`.
    pub sierra_replace_ids: Option<bool>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct TomlProfile {
    pub inherits: Option<SmolStr>,
    pub cairo: Option<TomlCairo>,
    pub tool: Option<ToolDefinition>,
}

impl DefaultForProfile for TomlProfile {
    fn default_for_profile(profile: &Profile) -> Self {
        let mut result = TomlProfile::default();
        let default_cairo: TomlCairo = ManifestCompilerConfig::default_for_profile(profile).into();
        result.cairo = Some(default_cairo);
        result
    }
}

impl TomlManifest {
    pub fn read_from_path(path: &Utf8Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read manifest at `{path}`"))?;

        Self::read_from_str(&contents)
            .with_context(|| format!("failed to parse manifest at `{path}`"))
    }

    pub fn read_from_str(contents: &str) -> Result<Self> {
        toml::from_str(contents).map_err(Into::into)
    }
}

impl TomlDependency {
    fn resolve(&self) -> DetailedTomlDependency {
        match self {
            TomlDependency::Simple(version) => DetailedTomlDependency {
                version: Some(version.clone()),
                ..Default::default()
            },
            TomlDependency::Detailed(detailed) => detailed.clone(),
        }
    }
}

impl TomlManifest {
    pub fn to_manifest(
        &self,
        manifest_path: &Utf8Path,
        source_id: SourceId,
        profile: Profile,
    ) -> Result<Manifest> {
        let root = manifest_path
            .parent()
            .expect("manifest path parent must always exist");

        let Some(package) = self.package.as_deref() else {
            bail!("no `package` section found");
        };

        let package_id = {
            let name = package.name.clone();
            let version = package.version.clone().to_version()?;
            PackageId::new(name, version, source_id)
        };

        let mut dependencies = Vec::new();
        for (name, toml_dep) in self.dependencies.iter().flatten() {
            dependencies.push(toml_dep.to_dependency(name.clone(), manifest_path)?);
        }

        let no_core = package.no_core.unwrap_or(false);

        let targets = self.collect_targets(package.name.to_smol_str(), root)?;

        let scripts: BTreeMap<SmolStr, ScriptDefinition> = self
            .scripts
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|(name, script)| -> Result<(SmolStr, ScriptDefinition)> {
                Ok((name, ScriptDefinition::from_str(&script)?))
            })
            .try_collect()?;

        let profile_definition = self.collect_profile_definition(profile.clone())?;
        let compiler_config = self.collect_compiler_config(&profile, profile_definition.clone())?;
        let tool = self.collect_tool(profile_definition)?;
        let profiles = self.collect_profiles()?;

        Ok(Manifest {
            summary: Summary::build(package_id)
                .with_dependencies(dependencies)
                .no_core(no_core)
                .finish(),
            targets,
            metadata: ManifestMetadata {
                authors: package.authors.clone(),
                urls: package.urls.clone(),
                description: package.description.clone(),
                documentation: package.documentation.clone(),
                homepage: package.homepage.clone(),
                keywords: package.keywords.clone(),
                license: package.license.clone(),
                license_file: package.license_file.clone(),
                readme: package.readme.clone(),
                repository: package.repository.clone(),
                tool_metadata: tool,
                cairo_version: package.cairo_version.clone(),
            },
            compiler_config,
            scripts,
            profiles,
        })
    }

    fn collect_targets(&self, package_name: SmolStr, root: &Utf8Path) -> Result<Vec<Target>> {
        let default_source_path = root.join(DEFAULT_SOURCE_PATH);

        let mut targets = Vec::new();

        if let Some(lib_toml) = &self.lib {
            let name = lib_toml
                .name
                .clone()
                .unwrap_or_else(|| package_name.clone());

            let target = Target::try_from_structured_params(
                Target::LIB,
                name,
                default_source_path.clone(),
                &lib_toml.params,
            )?;
            targets.push(target);
        }

        for (kind_toml, ext_toml) in self
            .target
            .iter()
            .flatten()
            .flat_map(|(k, vs)| vs.iter().map(|v| (k.clone(), v)))
        {
            let name = ext_toml
                .name
                .clone()
                .unwrap_or_else(|| package_name.clone());

            let target = Target::try_from_structured_params(
                kind_toml,
                name,
                default_source_path.clone(),
                &ext_toml.params,
            )?;
            targets.push(target);
        }

        Self::check_unique_targets(&targets, &package_name)?;

        if targets.is_empty() {
            trace!("manifest has no targets, assuming default `lib` target");
            let target = Target::without_params(Target::LIB, package_name, default_source_path);
            targets.push(target);
        }

        Ok(targets)
    }

    fn collect_profiles(&self) -> Result<Vec<Profile>> {
        if let Some(toml_profiles) = &self.profile {
            let mut result = Vec::new();
            for name in toml_profiles.keys() {
                let profile = Profile::new(name.clone())?;
                result.push(profile);
            }
            Ok(result)
        } else {
            Ok(vec![])
        }
    }

    fn collect_profile_definition(&self, profile: Profile) -> Result<TomlProfile> {
        let toml_cairo = self.cairo.clone().unwrap_or_default();

        let toml_profiles = self.profile.clone();
        let profile_definition = toml_profiles
            .clone()
            .unwrap_or_default()
            .get(profile.as_str())
            .cloned();

        let parent_profile = profile_definition
            .clone()
            .unwrap_or_default()
            .inherits
            .map(Profile::new)
            .unwrap_or_else(|| {
                if profile.is_custom() {
                    Ok(Profile::default())
                } else {
                    Ok(profile.clone())
                }
            })?;

        if parent_profile.is_custom() {
            bail!(
                "profile can inherit from `dev` or `release` only, found `{}`",
                parent_profile.as_str()
            );
        }

        let parent_default = TomlProfile::default_for_profile(&parent_profile);
        let parent_definition = toml_profiles
            .unwrap_or_default()
            .get(parent_profile.as_str())
            .cloned()
            .unwrap_or(parent_default.clone());

        let mut parent_definition = toml_merge(&parent_default, &parent_definition)?;

        let parent_cairo = toml_merge(&parent_definition.cairo, &toml_cairo)?;
        parent_definition.cairo = parent_cairo;

        let profile = if let Some(profile_definition) = profile_definition {
            toml_merge(&parent_definition, &profile_definition)?
        } else {
            parent_definition
        };

        Ok(profile)
    }

    fn collect_compiler_config(
        &self,
        profile: &Profile,
        profile_definition: TomlProfile,
    ) -> Result<ManifestCompilerConfig> {
        let mut compiler_config = ManifestCompilerConfig::default_for_profile(profile);
        if let Some(cairo) = profile_definition.cairo {
            if let Some(sierra_replace_ids) = cairo.sierra_replace_ids {
                compiler_config.sierra_replace_ids = sierra_replace_ids;
            }
        }
        Ok(compiler_config)
    }

    fn collect_tool(&self, profile_definition: TomlProfile) -> Result<Option<ToolDefinition>> {
        if let Some(tool) = &self.tool {
            if let Some(profile_tool) = &profile_definition.tool {
                toml_merge(tool, profile_tool).map(Some)
            } else {
                Ok(Some(tool.clone()))
            }
        } else {
            Ok(profile_definition.tool)
        }
    }

    fn check_unique_targets(targets: &[Target], package_name: &str) -> Result<()> {
        let mut used = HashSet::with_capacity(targets.len());
        for target in targets {
            if !used.insert((target.kind.as_str(), target.name.as_str())) {
                if target.name == package_name {
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

impl TomlDependency {
    fn to_dependency(
        &self,
        name: PackageName,
        manifest_path: &Utf8Path,
    ) -> Result<ManifestDependency> {
        self.resolve().to_dependency(name, manifest_path)
    }
}

impl DetailedTomlDependency {
    fn to_dependency(
        &self,
        name: PackageName,
        manifest_path: &Utf8Path,
    ) -> Result<ManifestDependency> {
        let version_req = self.version.to_owned().unwrap_or(VersionReq::STAR);

        if self.branch.is_some() || self.tag.is_some() || self.rev.is_some() {
            ensure!(
                self.git.is_some(),
                "dependency ({name}) is non-Git, but provides `branch`, `tag` or `rev`"
            );

            ensure!(
                [&self.branch, &self.tag, &self.rev]
                    .iter()
                    .filter(|o| o.is_some())
                    .count()
                    <= 1,
                "dependency ({name}) specification is ambiguous, \
                only one of `branch`, `tag` or `rev` is allowed"
            );
        }

        let source_id = match (self.version.as_ref(), self.git.as_ref(), self.path.as_ref()) {
            (None, None, None) => bail!(
                "dependency ({name}) must be specified providing a local path, Git repository, \
                or version to use"
            ),

            (_, Some(_), Some(_)) => bail!(
                "dependency ({name}) specification is ambiguous, \
                only one of `git` or `path` is allowed"
            ),

            (_, None, Some(path)) => {
                let root = manifest_path
                    .parent()
                    .expect("manifest path must always have parent");
                let path = root.join(path);
                let path = fsx::canonicalize(path)?;
                let path = path.try_as_utf8()?;
                SourceId::for_path(path)?
            }

            (_, Some(git), None) => {
                let reference = if let Some(branch) = &self.branch {
                    GitReference::Branch(branch.into())
                } else if let Some(tag) = &self.tag {
                    GitReference::Tag(tag.into())
                } else if let Some(rev) = &self.rev {
                    GitReference::Rev(rev.into())
                } else {
                    GitReference::DefaultBranch
                };

                SourceId::for_git(git, &reference)?
            }

            (Some(_), None, None) => SourceId::default(),
        };

        Ok(ManifestDependency {
            name,
            version_req,
            source_id,
        })
    }
}

/// Merge two `toml::Value` serializable structs.
pub fn toml_merge<'de, T, S>(target: &T, source: &S) -> Result<T>
where
    T: Serialize + Deserialize<'de>,
    S: Serialize + Deserialize<'de>,
{
    let mut params = toml::Value::try_from(target)?;
    let source = toml::Value::try_from(source)?;

    params.as_table_mut().unwrap().extend(
        source
            .as_table()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone())),
    );
    Ok(toml::Value::try_into(params)?)
}
