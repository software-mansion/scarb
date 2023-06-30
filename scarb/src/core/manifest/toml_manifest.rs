use std::collections::BTreeMap;
use std::default::Default;
use std::fs;

use anyhow::{anyhow, bail, ensure, Context, Result};
use camino::Utf8Path;
use itertools::Itertools;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use tracing::trace;
use url::Url;

use crate::compiler::{DefaultForProfile, Profile};
use crate::core::manifest::maybe_workspace::{MaybeWorkspace, WorkspaceInherit};
use crate::core::manifest::scripts::ScriptDefinition;
use crate::core::manifest::{ManifestDependency, ManifestMetadata, Summary, Target};
use crate::core::package::PackageId;
use crate::core::source::{GitReference, SourceId};
use crate::core::{ManifestBuilder, ManifestCompilerConfig, PackageName};
use crate::internal::serdex::{toml_merge, RelativeUtf8PathBuf};
use crate::internal::to_version::ToVersion;
use crate::{DEFAULT_SOURCE_PATH, MANIFEST_FILE_NAME};

use super::Manifest;

/// This type is used to deserialize `Scarb.toml` files.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TomlManifest {
    pub package: Option<Box<TomlPackage>>,
    pub workspace: Option<TomlWorkspace>,
    pub dependencies: Option<BTreeMap<PackageName, MaybeTomlWorkspaceDependency>>,
    pub lib: Option<TomlTarget<TomlLibTargetParams>>,
    pub cairo_plugin: Option<TomlTarget<TomlExternalTargetParams>>,
    pub target: Option<BTreeMap<TomlTargetKind, Vec<TomlTarget<TomlExternalTargetParams>>>>,
    pub cairo: Option<TomlCairo>,
    pub profile: Option<TomlProfilesDefinition>,
    pub scripts: Option<BTreeMap<SmolStr, MaybeWorkspaceScriptDefinition>>,
    pub tool: Option<BTreeMap<SmolStr, MaybeWorkspaceTomlTool>>,
}

type MaybeWorkspaceScriptDefinition = MaybeWorkspace<ScriptDefinition, WorkspaceScriptDefinition>;

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct WorkspaceScriptDefinition {
    pub workspace: bool,
}

impl WorkspaceInherit for WorkspaceScriptDefinition {
    fn inherit_toml_table(&self) -> &str {
        "scripts"
    }

    fn workspace(&self) -> bool {
        self.workspace
    }
}

type TomlProfilesDefinition = BTreeMap<SmolStr, TomlProfile>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TomlWorkspaceTool {
    pub workspace: bool,
}

impl WorkspaceInherit for TomlWorkspaceTool {
    fn inherit_toml_table(&self) -> &str {
        "tool"
    }

    fn workspace(&self) -> bool {
        self.workspace
    }
}

type MaybeWorkspaceTomlTool = MaybeWorkspace<toml::Value, TomlWorkspaceTool>;
type TomlToolsDefinition = BTreeMap<SmolStr, toml::Value>;

/// Represents the workspace root definition.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TomlWorkspace {
    pub members: Option<Vec<String>>,
    pub package: Option<TomlPackage>,
    pub dependencies: Option<BTreeMap<PackageName, TomlDependency>>,
    pub scripts: Option<BTreeMap<SmolStr, ScriptDefinition>>,
    pub tool: Option<TomlToolsDefinition>,
}

/// Represents the `package` section of a `Scarb.toml`.
#[derive(Debug, Clone, Deserialize, Serialize)]
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

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct TomlWorkspaceDependency {
    pub workspace: bool,
}

impl WorkspaceInherit for TomlWorkspaceDependency {
    fn inherit_toml_table(&self) -> &str {
        "dependencies"
    }

    fn workspace(&self) -> bool {
        self.workspace
    }
}

type MaybeTomlWorkspaceDependency = MaybeWorkspace<TomlDependency, TomlWorkspaceDependency>;

#[derive(Debug, Clone, Deserialize, Serialize)]
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
    pub path: Option<RelativeUtf8PathBuf>,

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

pub type TomlExternalTargetParams = BTreeMap<SmolStr, toml::Value>;

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
    pub tool: Option<TomlToolsDefinition>,
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
    pub fn is_package(&self) -> bool {
        self.package.is_some()
    }

    pub fn is_workspace(&self) -> bool {
        self.workspace.is_some()
    }

    pub fn get_workspace(&self) -> Option<TomlWorkspace> {
        self.workspace.as_ref().cloned()
    }

    pub fn fetch_workspace(&self) -> Result<TomlWorkspace> {
        self.get_workspace()
            .ok_or_else(|| anyhow!("manifest is not a workspace"))
    }

    pub fn to_manifest(
        &self,
        manifest_path: &Utf8Path,
        source_id: SourceId,
        profile: Profile,
        workspace_manifest: Option<&TomlManifest>,
    ) -> Result<Manifest> {
        let root = manifest_path
            .parent()
            .expect("manifest path parent must always exist");

        let Some(package) = self.package.as_deref() else {
            bail!("no `package` section found");
        };

        let toml_workspace = workspace_manifest.and_then(|m| m.workspace.clone());
        // For root package, no need to fetch workspace separately.
        let workspace = self
            .workspace
            .as_ref()
            .cloned()
            .or(toml_workspace)
            .unwrap_or_default();

        // Apply package defaults from workspace.
        let package = if let Some(workspace_package) = workspace.package {
            toml_merge(&workspace_package, package)?
        } else {
            package.clone()
        };

        let package_id = {
            let name = package.name.clone();
            let version = package.version.clone().to_version()?;
            // Override path dependencies with manifest path.
            let source_id = source_id
                .to_path()
                .map(|_| SourceId::for_path(manifest_path))
                .unwrap_or(Ok(source_id))?;
            PackageId::new(name, version, source_id)
        };

        let mut dependencies = Vec::new();
        for (name, toml_dep) in self.dependencies.iter().flatten() {
            let inherit_ws = || {
                workspace
                    .dependencies
                    .as_ref()
                    .and_then(|deps| deps.get(name.as_str()))
                    .cloned()
                    .ok_or_else(|| anyhow!("dependency `{}` not found in workspace", name.clone()))
            };
            let toml_dep = toml_dep.clone().resolve(name.as_str(), inherit_ws)?;
            dependencies.push(toml_dep.to_dependency(name.clone(), manifest_path)?);
        }

        let no_core = package.no_core.unwrap_or(false);

        let summary = Summary::builder()
            .package_id(package_id)
            .dependencies(dependencies)
            .no_core(no_core)
            .build();

        let targets = self.collect_targets(package.name.to_smol_str(), root)?;

        let scripts = self.scripts.clone().unwrap_or_default();

        let scripts: BTreeMap<SmolStr, ScriptDefinition> = scripts
            .into_iter()
            .map(|(name, script)| -> Result<(SmolStr, ScriptDefinition)> {
                let inherit_ws = || {
                    workspace
                        .scripts
                        .clone()
                        .and_then(|scripts| scripts.get(&name).cloned())
                        .ok_or_else(|| anyhow!("script `{}` not found in workspace", name.clone()))
                };
                Ok((name.clone(), script.resolve(name.as_str(), inherit_ws)?))
            })
            .try_collect()?;

        // Following Cargo convention, pull profile config from workspace root only.
        let profile_source = workspace_manifest.unwrap_or(self);
        let profile_definition = profile_source.collect_profile_definition(profile.clone())?;

        let compiler_config = self.collect_compiler_config(&profile, profile_definition.clone())?;
        let workspace_tool = workspace.tool.clone();
        let tool = self.collect_tool(profile_definition, workspace_tool)?;

        let metadata = ManifestMetadata {
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
            cairo_version: package.cairo_version,
        };

        let manifest = ManifestBuilder::default()
            .summary(summary)
            .targets(targets)
            .metadata(metadata)
            .compiler_config(compiler_config)
            .scripts(scripts)
            .build()?;

        Ok(manifest)
    }

    fn collect_targets(&self, package_name: SmolStr, root: &Utf8Path) -> Result<Vec<Target>> {
        let default_source_path = root.join(DEFAULT_SOURCE_PATH);

        let mut targets = Vec::new();

        targets.extend(Self::collect_target(
            Target::LIB,
            self.lib.as_ref(),
            &package_name,
            &default_source_path,
        )?);

        targets.extend(Self::collect_target(
            Target::CAIRO_PLUGIN,
            self.cairo_plugin.as_ref(),
            &package_name,
            &default_source_path,
        )?);

        for (kind_toml, ext_toml) in self
            .target
            .iter()
            .flatten()
            .flat_map(|(k, vs)| vs.iter().map(|v| (k.clone(), v)))
        {
            targets.extend(Self::collect_target(
                kind_toml,
                Some(ext_toml),
                &package_name,
                &default_source_path,
            )?);
        }

        if targets.is_empty() {
            trace!("manifest has no targets, assuming default `lib` target");
            let target = Target::without_params(Target::LIB, package_name, default_source_path);
            targets.push(target);
        }

        Ok(targets)
    }

    fn collect_target<T: Serialize>(
        kind: impl Into<SmolStr>,
        target: Option<&TomlTarget<T>>,
        default_name: &SmolStr,
        default_source_path: &Utf8Path,
    ) -> Result<Option<Target>> {
        let Some(target) = target else {
            return Ok(None);
        };

        let name = target.name.clone().unwrap_or_else(|| default_name.clone());

        let target = Target::try_from_structured_params(
            kind,
            name,
            default_source_path.to_path_buf(),
            &target.params,
        )?;

        Ok(Some(target))
    }

    pub fn collect_profiles(&self) -> Result<Vec<Profile>> {
        self.profile
            .as_ref()
            .map(|toml_profiles| {
                toml_profiles
                    .keys()
                    .map(|name| Profile::new(name.clone()))
                    .try_collect()
            })
            .unwrap_or(Ok(vec![]))
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

    fn collect_tool(
        &self,
        profile_definition: TomlProfile,
        workspace_tool: Option<TomlToolsDefinition>,
    ) -> Result<Option<TomlToolsDefinition>> {
        self.tool
            .clone()
            .map(|tool| {
                tool.iter()
                    .map(|(name, tool)| {
                        let inherit_ws = || {
                            workspace_tool
                                .clone()
                                .and_then(|tools| tools.get(name).cloned())
                                .ok_or_else(|| {
                                    anyhow!("tool `{}` not found in workspace tools", name.clone())
                                })
                        };
                        let value = tool.clone().resolve(name, inherit_ws)?;
                        Ok((name.clone(), value))
                    })
                    .collect::<Result<BTreeMap<SmolStr, toml::Value>>>()
            })
            .map_or(Ok(None), |v| v.map(Some))?
            .map(|tool| {
                if let Some(profile_tool) = &profile_definition.tool {
                    toml_merge(&tool, profile_tool)
                } else {
                    Ok(tool)
                }
            })
            .map_or(Ok(None), |v| v.map(Some))
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
                let path = path
                    .relative_to_file(manifest_path)?
                    .join(MANIFEST_FILE_NAME);
                SourceId::for_path(&path)?
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
