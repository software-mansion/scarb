use std::collections::BTreeMap;
use std::default::Default;
use std::fs;

use anyhow::{anyhow, bail, ensure, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use itertools::Itertools;
use pathdiff::diff_utf8_paths;
use semver::{Version, VersionReq};
use serde::{de, Deserialize, Serialize};
use serde_untagged::UntaggedEnumVisitor;
use smol_str::SmolStr;
use tracing::trace;
use url::Url;

use crate::compiler::{DefaultForProfile, Profile};
use crate::core::manifest::maybe_workspace::{MaybeWorkspace, WorkspaceInherit};
use crate::core::manifest::scripts::ScriptDefinition;
use crate::core::manifest::{ManifestDependency, ManifestMetadata, Summary, Target};
use crate::core::package::PackageId;
use crate::core::source::{GitReference, SourceId};
use crate::core::{
    DependencyVersionReq, ManifestBuilder, ManifestCompilerConfig, PackageName, TargetKind,
    TestTargetProps, TestTargetType,
};
use crate::internal::fsx;
use crate::internal::fsx::PathBufUtf8Ext;
use crate::internal::serdex::{toml_merge, RelativeUtf8PathBuf};
use crate::internal::to_version::ToVersion;
use crate::{
    DEFAULT_MODULE_MAIN_FILE, DEFAULT_SOURCE_PATH, DEFAULT_TESTS_PATH, MANIFEST_FILE_NAME,
};

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
    pub test: Option<Vec<TomlTarget<TomlExternalTargetParams>>>,
    pub target: Option<BTreeMap<TargetKind, Vec<TomlTarget<TomlExternalTargetParams>>>>,
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
    pub package: Option<PackageInheritableFields>,
    pub dependencies: Option<BTreeMap<PackageName, TomlDependency>>,
    pub scripts: Option<BTreeMap<SmolStr, ScriptDefinition>>,
    pub tool: Option<TomlToolsDefinition>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct PackageInheritableFields {
    pub version: Option<Version>,
    pub authors: Option<Vec<String>>,
    pub description: Option<String>,
    pub documentation: Option<String>,
    pub homepage: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub license: Option<String>,
    pub license_file: Option<String>,
    pub readme: Option<PathOrBool>,
    pub repository: Option<String>,
    pub cairo_version: Option<VersionReq>,
}

macro_rules! get_field {
    ($name:ident, $type:ty) => {
        pub fn $name(&self) -> Result<$type> {
            self.$name.clone().ok_or_else(|| {
                anyhow!(
                    "no `{}` field found in workspace definition",
                    stringify!($name)
                )
            })
        }
    };
}
type VecOfStrings = Vec<String>;

impl PackageInheritableFields {
    get_field!(version, Version);
    get_field!(authors, VecOfStrings);
    get_field!(keywords, VecOfStrings);
    get_field!(cairo_version, VersionReq);
    get_field!(description, String);
    get_field!(documentation, String);
    get_field!(homepage, String);
    get_field!(license, String);
    get_field!(license_file, String);
    get_field!(repository, String);

    pub fn readme(&self, workspace_root: &Utf8Path, package_root: &Utf8Path) -> Result<PathOrBool> {
        let Ok(Some(readme)) = readme_for_package(workspace_root, self.readme.as_ref()) else {
            bail!("`workspace.package.readme` was not defined");
        };
        diff_utf8_paths(
            workspace_root.parent().unwrap().join(readme),
            package_root.parent().unwrap(),
        )
        .map(PathOrBool::Path)
        .context("failed to create relative path to workspace readme")
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct TomlWorkspaceField {
    workspace: bool,
}

impl WorkspaceInherit for TomlWorkspaceField {
    fn inherit_toml_table(&self) -> &str {
        "package"
    }

    fn workspace(&self) -> bool {
        self.workspace
    }
}

type MaybeWorkspaceField<T> = MaybeWorkspace<T, TomlWorkspaceField>;

/// Represents the `package` section of a `Scarb.toml`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TomlPackage {
    pub name: PackageName,
    pub version: MaybeWorkspaceField<Version>,
    pub authors: Option<MaybeWorkspaceField<Vec<String>>>,
    pub urls: Option<BTreeMap<String, String>>,
    pub description: Option<MaybeWorkspaceField<String>>,
    pub documentation: Option<MaybeWorkspaceField<String>>,
    pub homepage: Option<MaybeWorkspaceField<String>>,
    pub keywords: Option<MaybeWorkspaceField<Vec<String>>>,
    pub license: Option<MaybeWorkspaceField<String>>,
    pub license_file: Option<MaybeWorkspaceField<String>>,
    pub readme: Option<MaybeWorkspaceField<PathOrBool>>,
    pub repository: Option<MaybeWorkspaceField<String>>,
    /// **UNSTABLE** This package does not depend on Cairo's `core`.
    pub no_core: Option<bool>,
    pub cairo_version: Option<MaybeWorkspaceField<VersionReq>>,
}

#[derive(Clone, Debug, Serialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum PathOrBool {
    Path(Utf8PathBuf),
    Bool(bool),
}

impl<'de> Deserialize<'de> for PathOrBool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        UntaggedEnumVisitor::new()
            .bool(|b| Ok(PathOrBool::Bool(b)))
            .string(|s| Ok(PathOrBool::Path(s.into())))
            .deserialize(deserializer)
    }
}

impl From<Utf8PathBuf> for PathOrBool {
    fn from(p: Utf8PathBuf) -> Self {
        Self::Path(p)
    }
}

impl From<bool> for PathOrBool {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
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
    /// [`VersionReq`] specified as a string, e.g. `package = "<version>"`.
    Simple(VersionReq),
    /// Detailed specification as a table, e.g. `package = { version = "<version>" }`.
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

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TomlTarget<P> {
    pub name: Option<SmolStr>,
    pub source_path: Option<Utf8PathBuf>,

    #[serde(flatten)]
    pub params: P,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TomlLibTargetParams {
    pub sierra: Option<bool>,
    pub casm: Option<bool>,
    pub sierra_text: Option<bool>,
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
            .with_context(|| format!("failed to read manifest at: {path}"))?;

        Self::read_from_str(&contents)
            .with_context(|| format!("failed to parse manifest at: {path}"))
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
        workspace_manifest_path: &Utf8Path,
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

        let inheritable_package = workspace.package.clone().unwrap_or_default();

        let package_id = {
            let name = package.name.clone();
            let version = package
                .version
                .clone()
                .resolve("version", || inheritable_package.version())?
                .to_version()?;
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
                    .ok_or_else(|| anyhow!("dependency `{}` not found in workspace", name.clone()))?
                    .to_dependency(name.clone(), workspace_manifest_path)
            };
            let toml_dep = toml_dep
                .clone()
                .map(|dep| dep.to_dependency(name.clone(), manifest_path))?
                .resolve(name.as_str(), inherit_ws)?;
            dependencies.push(toml_dep);
        }

        let no_core = package.no_core.unwrap_or(false);

        let targets = self.collect_targets(package.name.to_smol_str(), root)?;

        let summary = Summary::builder()
            .target_kinds(targets.iter().map(|t| t.kind.clone()).collect())
            .package_id(package_id)
            .dependencies(dependencies)
            .no_core(no_core)
            .build();

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
            urls: package.urls.clone(),
            tool_metadata: tool,
            authors: package
                .authors
                .clone()
                .map(|mw| mw.resolve("authors", || inheritable_package.authors()))
                .transpose()?,
            description: package
                .description
                .clone()
                .map(|mw| mw.resolve("description", || inheritable_package.description()))
                .transpose()?,
            documentation: package
                .documentation
                .clone()
                .map(|mw| mw.resolve("documentation", || inheritable_package.documentation()))
                .transpose()?,
            homepage: package
                .homepage
                .clone()
                .map(|mw| mw.resolve("homepage", || inheritable_package.homepage()))
                .transpose()?,
            keywords: package
                .keywords
                .clone()
                .map(|mw| mw.resolve("keywords", || inheritable_package.keywords()))
                .transpose()?,
            license: package
                .license
                .clone()
                .map(|mw| mw.resolve("license", || inheritable_package.license()))
                .transpose()?,
            license_file: package
                .license_file
                .clone()
                .map(|mw| mw.resolve("license_file", || inheritable_package.license_file()))
                .transpose()?,
            readme: readme_for_package(
                manifest_path,
                package
                    .readme
                    .clone()
                    .map(|mw| {
                        mw.resolve("readme", || {
                            inheritable_package.readme(workspace_manifest_path, manifest_path)
                        })
                    })
                    .transpose()?
                    .as_ref(),
            )?,
            repository: package
                .repository
                .clone()
                .map(|mw| mw.resolve("repository", || inheritable_package.repository()))
                .transpose()?,
            cairo_version: package
                .cairo_version
                .clone()
                .map(|mw| mw.resolve("cairo_version", || inheritable_package.cairo_version()))
                .transpose()?,
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
        let mut targets = Vec::new();

        targets.extend(Self::collect_target(
            TargetKind::LIB,
            self.lib.as_ref(),
            &package_name,
            root,
        )?);

        targets.extend(Self::collect_target(
            TargetKind::CAIRO_PLUGIN,
            self.cairo_plugin.as_ref(),
            &package_name,
            root,
        )?);

        for (kind, ext_toml) in self
            .target
            .iter()
            .flatten()
            .flat_map(|(k, vs)| vs.iter().map(|v| (k.clone(), v)))
        {
            targets.extend(Self::collect_target(
                kind,
                Some(ext_toml),
                &package_name,
                root,
            )?);
        }

        if targets.is_empty() {
            trace!("manifest has no targets, assuming default `lib` target");
            let default_source_path = root.join(DEFAULT_SOURCE_PATH);
            let target =
                Target::without_params(TargetKind::LIB, package_name.clone(), default_source_path);
            targets.push(target);
        }

        // Skip autodetect for cairo plugins.
        let auto_detect = !targets.iter().any(Target::is_cairo_plugin);
        targets.extend(self.collect_test_targets(package_name.clone(), root, auto_detect)?);

        Ok(targets)
    }

    fn collect_test_targets(
        &self,
        package_name: SmolStr,
        root: &Utf8Path,
        auto_detect: bool,
    ) -> Result<Vec<Target>> {
        let mut targets = Vec::new();
        if let Some(test) = self.test.as_ref() {
            // Read test targets from manifest file.
            for test_toml in test {
                targets.extend(Self::collect_target(
                    TargetKind::TEST,
                    Some(test_toml),
                    &package_name,
                    root,
                )?);
            }
        } else if auto_detect {
            // Auto-detect test target.
            let source_path = self.lib.as_ref().and_then(|l| l.source_path.clone());
            let target_name: SmolStr = format!("{package_name}_unittest").into();
            let target_config = TomlTarget::<TomlExternalTargetParams> {
                name: Some(target_name),
                source_path,
                params: TestTargetProps::default().try_into()?,
            };
            targets.extend(Self::collect_target::<TomlExternalTargetParams>(
                TargetKind::TEST,
                Some(&target_config),
                &package_name,
                root,
            )?);
            // Auto-detect test targets from `tests` directory.
            let tests_path = root.join(DEFAULT_TESTS_PATH);
            if tests_path.join(DEFAULT_MODULE_MAIN_FILE).exists() {
                // Tests directory contains `lib.cairo` file.
                // Treat whole tests directory as single module.
                let source_path = tests_path.join(DEFAULT_MODULE_MAIN_FILE);
                let target_name: SmolStr = format!("{package_name}_{DEFAULT_TESTS_PATH}").into();
                let target_config = TomlTarget::<TomlExternalTargetParams> {
                    name: Some(target_name),
                    source_path: Some(source_path),
                    params: TestTargetProps::new(TestTargetType::Integration).try_into()?,
                };
                targets.extend(Self::collect_target::<TomlExternalTargetParams>(
                    TargetKind::TEST,
                    Some(&target_config),
                    &package_name,
                    root,
                )?);
            } else {
                // Tests directory does not contain `lib.cairo` file.
                // Each file will be treated as separate crate.
                if let Ok(entries) = fs::read_dir(tests_path) {
                    for entry in entries.flatten() {
                        if !entry.file_type()?.is_file() {
                            continue;
                        }
                        let source_path = entry.path().try_into_utf8()?;
                        let file_stem = source_path.file_stem().unwrap().to_string();
                        let target_name: SmolStr = format!("{package_name}_{file_stem}").into();
                        let target_config = TomlTarget::<TomlExternalTargetParams> {
                            name: Some(target_name),
                            source_path: Some(source_path),
                            params: TestTargetProps::new(TestTargetType::Integration).try_into()?,
                        };
                        targets.extend(Self::collect_target(
                            TargetKind::TEST,
                            Some(&target_config),
                            &package_name,
                            root,
                        )?);
                    }
                }
            }
        };
        Ok(targets)
    }

    fn collect_target<T: Serialize>(
        kind: TargetKind,
        target: Option<&TomlTarget<T>>,
        default_name: &SmolStr,
        root: &Utf8Path,
    ) -> Result<Option<Target>> {
        let default_source_path = root.join(DEFAULT_SOURCE_PATH);
        let Some(target) = target else {
            return Ok(None);
        };

        if let Some(source_path) = &target.source_path {
            ensure!(
                kind == TargetKind::TEST || source_path == DEFAULT_SOURCE_PATH,
                "`{kind}` target cannot specify custom `source-path`"
            );
        }

        let name = target.name.clone().unwrap_or_else(|| default_name.clone());
        let source_path = target
            .source_path
            .clone()
            .map(|p| root.join_os(p))
            .map(fsx::canonicalize_utf8)
            .transpose()?
            .unwrap_or(default_source_path.to_path_buf());

        let target = Target::try_from_structured_params(kind, name, source_path, &target.params)?;

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
            .transpose()?
            .map(|tool| {
                if let Some(profile_tool) = &profile_definition.tool {
                    toml_merge(&tool, profile_tool)
                } else {
                    Ok(tool)
                }
            })
            .transpose()
    }
}

/// Returns the absolute canonical path of the README file for a [`TomlPackage`].
pub fn readme_for_package(
    package_root: &Utf8Path,
    readme: Option<&PathOrBool>,
) -> Result<Option<Utf8PathBuf>> {
    let file_name = match readme {
        None => default_readme_from_package_root(package_root.parent().unwrap()),
        Some(PathOrBool::Path(p)) => Some(p.as_path()),
        Some(PathOrBool::Bool(true)) => {
            default_readme_from_package_root(package_root.parent().unwrap())
                .or_else(|| Some("README.md".into()))
        }
        Some(PathOrBool::Bool(false)) => None,
    };

    abs_canonical_path(package_root, file_name)
}

/// Creates the absolute canonical path of the README file and checks if it exists
fn abs_canonical_path(prefix: &Utf8Path, readme: Option<&Utf8Path>) -> Result<Option<Utf8PathBuf>> {
    match readme {
        None => Ok(None),
        Some(readme) => {
            let path = prefix.parent().unwrap().join(readme);
            let path = fsx::canonicalize_utf8(&path)
                .with_context(|| format!("failed to find the readme at {path}"))?;
            Ok(Some(path))
        }
    }
}

const DEFAULT_README_FILES: &[&str] = &["README.md", "README.txt", "README"];

/// Checks if a file with any of the default README file names exists in the package root.
/// If so, returns a `Utf8Path` with that name.
fn default_readme_from_package_root(package_root: &Utf8Path) -> Option<&Utf8Path> {
    for &readme_filename in DEFAULT_README_FILES {
        if package_root.join(readme_filename).is_file() {
            return Some(readme_filename.into());
        }
    }
    None
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
        let version_req = self
            .version
            .to_owned()
            .map(DependencyVersionReq::from)
            .unwrap_or_default();

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

        Ok(ManifestDependency::builder()
            .name(name)
            .source_id(source_id)
            .version_req(version_req)
            .build())
    }
}
