use super::{FeatureName, Manifest};
use crate::compiler::{DefaultForProfile, Profile};
use crate::core::manifest::maybe_workspace::{MaybeWorkspace, WorkspaceInherit};
use crate::core::manifest::scripts::ScriptDefinition;
use crate::core::manifest::{ManifestDependency, ManifestMetadata, Summary, Target};
use crate::core::package::PackageId;
use crate::core::registry::{DEFAULT_REGISTRY_INDEX, DEFAULT_REGISTRY_INDEX_PATCH_SOURCE};
use crate::core::source::{GitReference, SourceId};
use crate::core::{
    Config, DepKind, DependencyVersionReq, EnabledFeature, InliningStrategy, ManifestBuilder,
    ManifestCompilerConfig, PackageName, TargetKind, TestTargetProps, TestTargetType,
};
use crate::internal::fsx;
use crate::internal::fsx::PathBufUtf8Ext;
use crate::internal::serdex::{RelativeUtf8PathBuf, toml_merge, toml_merge_apply_strategy};
use crate::internal::to_version::ToVersion;
use crate::sources::canonical_url::CanonicalUrl;
use crate::{
    DEFAULT_MODULE_MAIN_FILE, DEFAULT_SOURCE_PATH, DEFAULT_TESTS_PATH, MANIFEST_FILE_NAME,
};
use anyhow::{Context, Result, anyhow, bail, ensure};
use cairo_lang_filesystem::db::Edition;
use cairo_lang_filesystem::ids::CAIRO_FILE_EXTENSION;
use camino::{Utf8Path, Utf8PathBuf};
use indoc::{formatdoc, indoc};
use itertools::Itertools;
use pathdiff::diff_utf8_paths;
use scarb_ui::Ui;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize, de};
use serde_untagged::UntaggedEnumVisitor;
use smol_str::SmolStr;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::default::Default;
use std::fs;
use std::iter::{repeat, zip};
use std::ops::Deref;
use tracing::trace;
use url::Url;

/// This type is used to deserialize `Scarb.toml` files.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TomlManifest {
    pub package: Option<Box<TomlPackage>>,
    pub workspace: Option<TomlWorkspace>,
    pub dependencies: Option<BTreeMap<PackageName, MaybeWorkspaceTomlDependency>>,
    pub dev_dependencies: Option<BTreeMap<PackageName, MaybeWorkspaceTomlDependency>>,
    pub lib: Option<TomlTarget<TomlLibTargetParams>>,
    pub executable: Option<TomlTarget<TomlExecutableTargetParams>>,
    pub cairo_plugin: Option<TomlTarget<TomlCairoPluginTargetParams>>,
    pub test: Option<Vec<TomlTarget<TomlExternalTargetParams>>>,
    pub target: Option<BTreeMap<TargetKind, Vec<TomlTarget<TomlExternalTargetParams>>>>,
    pub cairo: Option<TomlCairo>,
    pub profile: Option<TomlProfilesDefinition>,
    pub scripts: Option<BTreeMap<SmolStr, MaybeWorkspaceScriptDefinition>>,
    pub tool: Option<BTreeMap<SmolStr, MaybeWorkspaceTomlTool>>,
    pub features: Option<BTreeMap<FeatureName, Vec<TomlFeatureToEnable>>>,
    pub patch: Option<BTreeMap<SmolStr, BTreeMap<PackageName, TomlDependency>>>,
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
    pub dev_dependencies: Option<BTreeMap<PackageName, TomlDependency>>,
    pub scripts: Option<BTreeMap<SmolStr, ScriptDefinition>>,
    pub tool: Option<TomlToolsDefinition>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct PackageInheritableFields {
    pub version: Option<Version>,
    pub edition: Option<Edition>,
    pub authors: Option<Vec<String>>,
    pub description: Option<String>,
    pub documentation: Option<String>,
    pub homepage: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub license: Option<String>,
    pub license_file: Option<Utf8PathBuf>,
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
    get_field!(license_file, Utf8PathBuf);
    get_field!(repository, String);
    get_field!(edition, Edition);

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
    pub edition: Option<MaybeWorkspaceField<Edition>>,
    pub publish: Option<bool>,
    pub authors: Option<MaybeWorkspaceField<Vec<String>>>,
    pub urls: Option<BTreeMap<String, String>>,
    pub description: Option<MaybeWorkspaceField<String>>,
    pub documentation: Option<MaybeWorkspaceField<String>>,
    pub homepage: Option<MaybeWorkspaceField<String>>,
    pub keywords: Option<MaybeWorkspaceField<Vec<String>>>,
    pub license: Option<MaybeWorkspaceField<String>>,
    pub license_file: Option<MaybeWorkspaceField<Utf8PathBuf>>,
    pub readme: Option<MaybeWorkspaceField<PathOrBool>>,
    pub repository: Option<MaybeWorkspaceField<String>>,
    pub include: Option<Vec<Utf8PathBuf>>,
    /// **UNSTABLE** This package does not depend on Cairo's `core`.
    pub no_core: Option<bool>,
    pub cairo_version: Option<MaybeWorkspaceField<VersionReq>>,
    pub experimental_features: Option<Vec<SmolStr>>,
    pub re_export_cairo_plugins: Option<Vec<PackageName>>,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(try_from = "serdex::MaybeWorkspaceTomlDependency")]
pub struct MaybeWorkspaceTomlDependency(MaybeWorkspace<TomlDependency, TomlWorkspaceDependency>);

impl From<MaybeWorkspace<TomlDependency, TomlWorkspaceDependency>>
    for MaybeWorkspaceTomlDependency
{
    fn from(dep: MaybeWorkspace<TomlDependency, TomlWorkspaceDependency>) -> Self {
        Self(dep)
    }
}

impl AsRef<MaybeWorkspace<TomlDependency, TomlWorkspaceDependency>>
    for MaybeWorkspaceTomlDependency
{
    fn as_ref(&self) -> &MaybeWorkspace<TomlDependency, TomlWorkspaceDependency> {
        &self.0
    }
}

impl Deref for MaybeWorkspaceTomlDependency {
    type Target = MaybeWorkspace<TomlDependency, TomlWorkspaceDependency>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

mod serdex {
    use crate::core::{
        DetailedTomlDependency, MaybeWorkspace, TomlDependency, TomlWorkspaceDependency,
    };
    use anyhow::ensure;
    use semver::VersionReq;
    use serde::{Deserialize, Deserializer, de};
    use serde_untagged::UntaggedEnumVisitor;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Detailed {
        pub workspace: Option<bool>,
        #[serde(flatten)]
        pub detailed: DetailedTomlDependency,
    }

    /// This is equivalent to `MaybeWorkspace<TomlDependency, TomlWorkspaceDependency>`, but we
    /// coalesce `DetailedTomlDependency` and `TomlWorkspaceDependency` to be able to validate them
    /// during deserialization and emit easy to understand errors.
    pub enum MaybeWorkspaceTomlDependency {
        Simple(VersionReq),
        Detailed(Box<Detailed>),
    }

    impl<'de> Deserialize<'de> for MaybeWorkspaceTomlDependency {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            UntaggedEnumVisitor::new()
                .string(|value| {
                    VersionReq::parse(value)
                        .map(MaybeWorkspaceTomlDependency::Simple)
                        .map_err(de::Error::custom)
                })
                .map(|map| {
                    map.deserialize()
                        .map(MaybeWorkspaceTomlDependency::Detailed)
                })
                .deserialize(deserializer)
        }
    }

    impl TryFrom<MaybeWorkspaceTomlDependency> for super::MaybeWorkspaceTomlDependency {
        type Error = anyhow::Error;
        fn try_from(value: MaybeWorkspaceTomlDependency) -> Result<Self, Self::Error> {
            Ok(match value {
                MaybeWorkspaceTomlDependency::Simple(simple) => {
                    Self(MaybeWorkspace::Defined(TomlDependency::Simple(simple)))
                }
                MaybeWorkspaceTomlDependency::Detailed(detailed) => {
                    if let Some(workspace) = detailed.workspace {
                        ensure!(
                            detailed.detailed.version.is_none(),
                            "field `version` is not allowed when inheriting workspace dependency"
                        );
                        ensure!(
                            detailed.detailed.path.is_none(),
                            "field `path` is not allowed when inheriting workspace dependency"
                        );
                        ensure!(
                            detailed.detailed.git.is_none(),
                            "field `git` is not allowed when inheriting workspace dependency"
                        );
                        ensure!(
                            detailed.detailed.branch.is_none(),
                            "field `branch` is not allowed when inheriting workspace dependency"
                        );
                        ensure!(
                            detailed.detailed.tag.is_none(),
                            "field `tag` is not allowed when inheriting workspace dependency"
                        );
                        ensure!(
                            detailed.detailed.rev.is_none(),
                            "field `rev` is not allowed when inheriting workspace dependency"
                        );
                        ensure!(
                            detailed.detailed.registry.is_none(),
                            "field `registry` is not allowed when inheriting workspace dependency"
                        );
                        ensure!(
                            detailed.detailed.default_features.is_none(),
                            "field `default-features` is not allowed when inheriting workspace dependency"
                        );
                        Self(MaybeWorkspace::Workspace(TomlWorkspaceDependency {
                            workspace,
                            features: detailed.detailed.features,
                        }))
                    } else {
                        Self(MaybeWorkspace::Defined(TomlDependency::Detailed(Box::new(
                            detailed.detailed,
                        ))))
                    }
                }
            })
        }
    }
}

/// When { workspace = true } you cannot define other keys that configure the source of
/// the dependency such as `version`, `registry`, `path`, `git`, `branch`, `tag`, `rev`.
/// You can also not define `default-features`.
/// Only `features` is allowed.
#[derive(Debug, Default, Clone, Serialize)]
pub struct TomlWorkspaceDependency {
    pub workspace: bool,
    pub features: Option<Vec<SmolStr>>,
}

impl WorkspaceInherit for TomlWorkspaceDependency {
    fn inherit_toml_table(&self) -> &str {
        "dependencies"
    }

    fn workspace(&self) -> bool {
        self.workspace
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TomlDependency {
    /// [`VersionReq`] specified as a string, e.g. `package = "<version>"`.
    Simple(VersionReq),
    /// Detailed specification as a table, e.g. `package = { version = "<version>" }`.
    Detailed(Box<DetailedTomlDependency>),
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

    pub registry: Option<Url>,

    pub default_features: Option<bool>,
    pub features: Option<Vec<SmolStr>>,
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

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TomlExecutableTargetParams {
    /// If true, will allow syscalls in the program.
    ///
    /// In general, syscalls are not allowed in executables, as they are currently not checked.
    pub allow_syscalls: Option<bool>,
    pub function: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TomlCairoPluginTargetParams {
    pub builtin: Option<bool>,
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
    /// Do not exit with error on compiler warnings.
    pub allow_warnings: Option<bool>,
    /// Enable auto gas withdrawal and gas usage check.
    pub enable_gas: Option<bool>,
    /// Add a mapping between sierra statement indexes and fully qualified paths of cairo functions
    /// to debug info. A statement index maps to a vector consisting of a function which caused the
    /// statement to be generated and all functions that were inlined or generated along the way.
    /// Used by [cairo-profiler](https://github.com/software-mansion/cairo-profiler).
    /// This feature is unstable and is subject to change.
    pub unstable_add_statements_functions_debug_info: Option<bool>,
    /// Add a mapping between sierra statement indexes and lines in cairo code
    /// to debug info. A statement index maps to a vector consisting of a line which caused the
    /// statement to be generated and all lines that were inlined or generated along the way.
    /// Used by [cairo-coverage](https://github.com/software-mansion/cairo-coverage).
    /// This feature is unstable and is subject to change.
    pub unstable_add_statements_code_locations_debug_info: Option<bool>,
    /// Whether to add panic backtrace handling to the generated code.
    pub panic_backtrace: Option<bool>,
    /// Do not generate panic handling code. This might be useful for client side proving.
    pub unsafe_panic: Option<bool>,
    /// Inlining strategy.
    pub inlining_strategy: Option<InliningStrategy>,
    /// Whether to enable incremental compilation.
    pub incremental: Option<bool>,
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

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct TomlToolScarbMetadata {
    pub allow_prebuilt_plugins: Option<Vec<String>>,
}

const DEPENDENCY_FEATURE_SEPARATOR: &str = "/";

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct TomlFeatureToEnable(String);

impl From<EnabledFeature> for TomlFeatureToEnable {
    fn from(value: EnabledFeature) -> Self {
        if let Some(package_name) = value.package {
            Self(format!(
                "{}{}{}",
                package_name, DEPENDENCY_FEATURE_SEPARATOR, value.feature
            ))
        } else {
            Self(value.feature.to_string())
        }
    }
}

impl TryFrom<TomlFeatureToEnable> for EnabledFeature {
    type Error = anyhow::Error;

    fn try_from(value: TomlFeatureToEnable) -> Result<Self, Self::Error> {
        let Some((package, feature)) = value.0.split_once(DEPENDENCY_FEATURE_SEPARATOR) else {
            return Ok(Self {
                package: None,
                feature: FeatureName::try_new(value.0)?,
            });
        };
        ensure!(
            !feature.contains(DEPENDENCY_FEATURE_SEPARATOR),
            formatdoc! {r#"
                    feature `{feature}` for package `{package}` contains invalid character `{DEPENDENCY_FEATURE_SEPARATOR}`
                    help: you can use `{DEPENDENCY_FEATURE_SEPARATOR}` to separate package name from feature name
                "#}
        );

        let context = || {
            formatdoc! {r#"
                    failed to deserialize package name from `{package}{DEPENDENCY_FEATURE_SEPARATOR}{feature}`
                    help: you can use `{DEPENDENCY_FEATURE_SEPARATOR}` to separate package name from feature name
                "#}
        };

        let package = Some(PackageName::try_new(package).with_context(context)?);
        let feature = FeatureName::try_new(feature).with_context(context)?;
        Ok(Self { feature, package })
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
    fn resolve(&self) -> Cow<'_, DetailedTomlDependency> {
        match self {
            TomlDependency::Simple(version) => Cow::Owned(DetailedTomlDependency {
                version: Some(version.clone()),
                ..Default::default()
            }),
            TomlDependency::Detailed(detailed) => Cow::Borrowed(detailed),
        }
    }

    /// Rewrite the dependency spec with provided features list.
    pub fn with_features(&self, features: Vec<SmolStr>) -> DetailedTomlDependency {
        let mut dep = self.resolve().into_owned();
        dep.features = Some(features);
        dep
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

    pub fn to_manifest(
        &self,
        manifest_path: &Utf8Path,
        workspace_manifest_path: &Utf8Path,
        source_id: SourceId,
        profile: Profile,
        workspace_manifest: Option<&TomlManifest>,
        config: &Config,
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
        let toml_deps = zip(self.dependencies.iter().flatten(), repeat(DepKind::Normal));
        let toml_dev_deps = zip(
            self.dev_dependencies.iter().flatten(),
            repeat(DepKind::Target(TargetKind::TEST)),
        );
        let all_deps = toml_deps.chain(toml_dev_deps);

        for ((name, toml_dep), kind) in all_deps {
            let inherit_ws = || {
                let ws_dep = workspace
                    .dependencies
                    .as_ref()
                    .and_then(|deps| deps.get(name.as_str()))
                    .cloned()
                    .ok_or_else(|| {
                        anyhow!("dependency `{}` not found in workspace", name.clone())
                    })?;

                // If `TomlWorkspaceDependency` declares `features` list,
                // extend the inherited ws dependency with this list instead of the shared ws one.
                let dep = match toml_dep.as_ref() {
                    MaybeWorkspace::Workspace(w) => w.features.as_ref().map(|features| {
                        let ws_features = match &ws_dep {
                            TomlDependency::Detailed(detailed) => {
                                detailed.features.clone().unwrap_or_default()
                            }
                            TomlDependency::Simple(_) => Vec::new(),
                        };
                        let features = features
                            .clone()
                            .into_iter()
                            .chain(ws_features.into_iter())
                            .sorted()
                            .dedup()
                            .collect();
                        ws_dep.with_features(features)
                    }),
                    _ => None,
                };

                dep.map(|dep| {
                    dep.to_dependency(name.clone(), workspace_manifest_path, kind.clone())
                })
                .unwrap_or_else(|| {
                    ws_dep.to_dependency(name.clone(), workspace_manifest_path, kind.clone())
                })
            };
            let toml_dep = toml_dep
                .clone()
                .as_ref()
                .clone()
                .map(|dep| dep.to_dependency(name.clone(), manifest_path, kind.clone()))?
                .resolve(name.as_str(), inherit_ws)?;
            dependencies.push(toml_dep);
        }

        if self.patch.is_some() {
            ensure!(
                workspace_manifest_path == manifest_path,
                "the `[patch]` section can only be defined in the workspace root manifests\nsection found in manifest: `{}`\nworkspace root manifest: `{}`",
                manifest_path,
                workspace_manifest_path
            );
        };

        let no_core = package.no_core.unwrap_or(false);

        let targets = self.collect_targets(package.name.to_smol_str(), root, config.ui())?;

        let publish = package.publish.unwrap_or(true);

        let re_export_cairo_plugins = package.re_export_cairo_plugins.clone().unwrap_or_default();

        let summary = Summary::builder()
            .package_id(package_id)
            .dependencies(dependencies)
            .re_export_cairo_plugins(re_export_cairo_plugins)
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
        let profile_source = if let Some(workspace_manifest) = workspace_manifest {
            let warn_msg = |section: &str| {
                config.ui().warn(formatdoc!(
                    r#"
                    in context of a workspace, only the `{section}` set in the workspace manifest is applied,
                    but the `{package}` package also defines `{section}` in the manifest
                    "#, package = package_id.name.as_str()
                ))
            };
            let is_root_package = manifest_path == workspace_manifest_path;
            if !is_root_package && self.cairo.is_some() {
                warn_msg("profile");
            }
            if !is_root_package && self.profile.is_some() {
                warn_msg("profile");
            }
            workspace_manifest
        } else {
            self
        };
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
                .map(|mw| match mw {
                    MaybeWorkspace::Defined(license_rel_path) => {
                        abs_canonical_path("license", manifest_path, &license_rel_path)
                    }
                    MaybeWorkspace::Workspace(_) => mw.resolve("license_file", || {
                        abs_canonical_path(
                            "license",
                            workspace_manifest_path,
                            &inheritable_package.license_file()?,
                        )
                    }),
                })
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
            include: package.include.clone(),
            cairo_version: package
                .cairo_version
                .clone()
                .map(|mw| mw.resolve("cairo_version", || inheritable_package.cairo_version()))
                .transpose()?,
        };

        let edition = package
            .edition
            .clone()
            .map(|edition| edition.resolve("edition", || inheritable_package.edition()))
            .transpose()?
            .unwrap_or_else(|| {
                if !targets.iter().any(Target::is_cairo_plugin) {
                    config.ui().warn(format!(
                        "`edition` field not set in `[package]` section for package `{}`",
                        package_id.name
                    ));
                }
                Edition::default()
            });

        // TODO (#1040): add checking for fields that are not present in ExperimentalFeaturesConfig
        let experimental_features = package.experimental_features.clone();

        let features = self.features.clone().unwrap_or_default();

        let manifest = ManifestBuilder::default()
            .summary(summary)
            .targets(targets)
            .publish(publish)
            .edition(edition)
            .metadata(metadata)
            .compiler_config(compiler_config)
            .scripts(scripts)
            .experimental_features(experimental_features)
            .features(features.try_into()?)
            .build()?;
        Ok(manifest)
    }

    fn collect_targets(
        &self,
        package_name: SmolStr,
        root: &Utf8Path,
        ui: Ui,
    ) -> Result<Vec<Target>> {
        let mut targets = Vec::new();

        targets.extend(Self::collect_target(
            TargetKind::LIB,
            self.lib.as_ref(),
            &package_name,
            root,
            None,
        )?);

        targets.extend(Self::collect_target(
            TargetKind::CAIRO_PLUGIN,
            self.cairo_plugin.as_ref(),
            &package_name,
            root,
            None,
        )?);

        targets.extend(Self::collect_target(
            TargetKind::EXECUTABLE,
            self.executable.as_ref(),
            &package_name,
            root,
            None,
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
                None,
            )?);
        }

        if targets.is_empty() {
            trace!("manifest has no targets, assuming default `lib` target");
            let default_source_path = root.join(DEFAULT_SOURCE_PATH.as_path());
            let target =
                Target::without_params(TargetKind::LIB, package_name.clone(), default_source_path);
            targets.push(target);
        }

        // Skip autodetect for cairo plugins.
        let auto_detect = !targets.iter().any(Target::is_cairo_plugin);
        self.collect_test_targets(&mut targets, package_name.clone(), root, auto_detect)?;

        Self::validate_targets(&targets, ui)?;

        Ok(targets)
    }

    fn validate_targets(targets: &[Target], ui: Ui) -> Result<()> {
        // Validate executable targets.
        let executable_functions = targets
            .iter()
            .filter(|target| target.kind == TargetKind::EXECUTABLE)
            .map(|target| target.params.get("function").cloned())
            .collect_vec();
        let unspecified = executable_functions.iter().any(Option::is_none);
        let specified = executable_functions
            .iter()
            .filter_map(|func| func.as_ref().and_then(|v| v.as_str()))
            .unique()
            .count();
        if unspecified && specified > 1 {
            ui.warn(indoc! {r#"
                you have specified multiple executable targets
                some of them specify different `function` names, some do not specify `function` name at all
                this is probably a mistake
                if your project defines more than one executable function, you need to specify `function` name

            "#})
        }
        Ok(())
    }

    fn collect_test_targets(
        &self,
        targets: &mut Vec<Target>,
        package_name: SmolStr,
        root: &Utf8Path,
        auto_detect: bool,
    ) -> Result<()> {
        if let Some(test) = self.test.as_ref() {
            // Read test targets from a manifest file.
            for test_toml in test {
                targets.extend(Self::collect_target(
                    TargetKind::TEST,
                    Some(test_toml),
                    &package_name,
                    root,
                    None,
                )?);
            }
        } else if auto_detect {
            // Auto-detect test target.
            let external_contracts = targets
                .iter()
                .filter(|target| target.kind == TargetKind::STARKNET_CONTRACT)
                .filter_map(|target| target.params.get("build-external-contracts"))
                .filter_map(|value| value.as_array())
                .flatten()
                .filter_map(|value| value.as_str().map(ToString::to_string))
                .sorted()
                .dedup()
                .collect_vec();
            let source_path = self.lib.as_ref().and_then(|l| l.source_path.clone());
            let target_name: SmolStr = format!("{package_name}_unittest").into();
            let target_config = TomlTarget::<TomlExternalTargetParams> {
                name: Some(target_name),
                source_path,
                params: TestTargetProps::default()
                    .with_build_external_contracts(external_contracts.clone())
                    .try_into()?,
            };
            let external_contracts = external_contracts
                .into_iter()
                .chain(vec![format!("{package_name}::*")])
                .sorted()
                .dedup()
                .collect_vec();
            targets.extend(Self::collect_target::<TomlExternalTargetParams>(
                TargetKind::TEST,
                Some(&target_config),
                &package_name,
                root,
                None,
            )?);
            // Auto-detect test targets from `tests` directory.
            let tests_path = root.join(DEFAULT_TESTS_PATH);
            let integration_target_config = |target_name, source_path| {
                let result: Result<TomlTarget<TomlExternalTargetParams>> =
                    Ok(TomlTarget::<TomlExternalTargetParams> {
                        name: Some(target_name),
                        source_path: Some(source_path),
                        params: TestTargetProps::new(TestTargetType::Integration)
                            .with_build_external_contracts(external_contracts.clone())
                            .try_into()?,
                    });
                result
            };
            if tests_path.join(DEFAULT_MODULE_MAIN_FILE).exists() {
                // Tests directory contains a `lib.cairo` file.
                // Treat the whole tests directory as a single module.
                let source_path = tests_path.join(DEFAULT_MODULE_MAIN_FILE);
                let target_name: SmolStr = format!("{package_name}_{DEFAULT_TESTS_PATH}").into();
                let target_config = integration_target_config(target_name, source_path)?;
                targets.extend(Self::collect_target::<TomlExternalTargetParams>(
                    TargetKind::TEST,
                    Some(&target_config),
                    &package_name,
                    root,
                    None,
                )?);
            } else {
                // Tests directory does not contain a `lib.cairo` file.
                // Each file will be treated as a separate crate.
                if let Ok(entries) = fs::read_dir(tests_path) {
                    for entry in entries.flatten() {
                        if !entry.file_type()?.is_file() {
                            continue;
                        }
                        let source_path = entry.path().try_into_utf8()?;
                        if source_path
                            .extension()
                            .map(|ext| ext != CAIRO_FILE_EXTENSION)
                            .unwrap_or(false)
                        {
                            trace!(
                                "ignoring non-cairo file {} from tests",
                                source_path.file_name().unwrap_or_default()
                            );
                            continue;
                        }
                        let file_stem = source_path.file_stem().unwrap().to_string();
                        let target_name: SmolStr = format!("{package_name}_{file_stem}").into();
                        let target_config = integration_target_config(target_name, source_path)?;
                        targets.extend(Self::collect_target(
                            TargetKind::TEST,
                            Some(&target_config),
                            &package_name,
                            root,
                            Some(format!("{package_name}_integrationtest").into()),
                        )?);
                    }
                }
            }
        };
        Ok(())
    }

    fn collect_target<T: Serialize>(
        kind: TargetKind,
        target: Option<&TomlTarget<T>>,
        default_name: &SmolStr,
        root: &Utf8Path,
        group_id: Option<SmolStr>,
    ) -> Result<Option<Target>> {
        let default_source_path = root.join(DEFAULT_SOURCE_PATH.as_path());
        let Some(target) = target else {
            return Ok(None);
        };

        if let Some(source_path) = &target.source_path {
            ensure!(
                kind == TargetKind::TEST || source_path == DEFAULT_SOURCE_PATH.as_path(),
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

        let target =
            Target::try_from_structured_params(kind, name, source_path, group_id, &target.params)?;

        Ok(Some(target))
    }

    pub fn collect_profiles(&self) -> Result<Vec<Profile>> {
        self.profile
            .as_ref()
            .map(|toml_profiles| {
                toml_profiles
                    .keys()
                    .cloned()
                    .map(Profile::try_new)
                    .try_collect()
            })
            .unwrap_or(Ok(vec![]))
    }

    fn collect_profile_definition(&self, profile: Profile) -> Result<TomlProfile> {
        let toml_cairo = self.cairo.clone().unwrap_or_default();
        let all_toml_profiles = self.profile.as_ref();

        let profile_definition =
            all_toml_profiles.and_then(|profiles| profiles.get(profile.as_str()).cloned());

        let parent_profile = profile_definition
            .as_ref()
            .and_then(|p| p.inherits.clone())
            .map(Profile::try_new)
            .unwrap_or_else(|| {
                if profile.is_custom() {
                    Ok(Profile::default())
                } else {
                    // Default profiles do not inherit from any other profile.
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
        let parent_definition = all_toml_profiles
            .and_then(|profiles| profiles.get(parent_profile.as_str()).cloned())
            .unwrap_or(parent_default.clone());

        let mut parent_definition = merge_profile(&parent_default, &parent_definition)?;
        let parent_cairo = toml_merge(&parent_definition.cairo, &toml_cairo)?;
        parent_definition.cairo = parent_cairo;

        let profile = if let Some(profile_definition) = profile_definition {
            merge_profile(&parent_definition, &profile_definition)?
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
            if let Some(inlining_strategy) = cairo.inlining_strategy {
                compiler_config.inlining_strategy = inlining_strategy;
            }
            if let Some(allow_warnings) = cairo.allow_warnings {
                compiler_config.allow_warnings = allow_warnings;
            }
            if let Some(enable_gas) = cairo.enable_gas {
                compiler_config.enable_gas = enable_gas;
            }
            if let Some(unstable_add_statements_functions_debug_info) =
                cairo.unstable_add_statements_functions_debug_info
            {
                compiler_config.unstable_add_statements_functions_debug_info =
                    unstable_add_statements_functions_debug_info;
            }
            if let Some(unstable_add_statements_code_locations_debug_info) =
                cairo.unstable_add_statements_code_locations_debug_info
            {
                compiler_config.unstable_add_statements_code_locations_debug_info =
                    unstable_add_statements_code_locations_debug_info;
            }
            if let Some(panic_backtrace) = cairo.panic_backtrace {
                compiler_config.panic_backtrace = panic_backtrace;
            }
            if let Some(unsafe_panic) = cairo.unsafe_panic {
                compiler_config.unsafe_panic = unsafe_panic;
            }
            if let Some(incremental) = cairo.incremental {
                compiler_config.incremental = incremental;
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
                    toml_merge_apply_strategy(&tool, profile_tool)
                } else {
                    Ok(tool)
                }
            })
            .transpose()
    }

    pub fn collect_patch(
        &self,
        manifest_path: &Utf8Path,
    ) -> Result<BTreeMap<CanonicalUrl, Vec<ManifestDependency>>> {
        if let Some(patch) = self.patch.clone() {
            let default_index_patch_source =
                SmolStr::new_static(DEFAULT_REGISTRY_INDEX_PATCH_SOURCE);
            ensure!(
                !(patch.contains_key(&default_index_patch_source)
                    && patch.contains_key(DEFAULT_REGISTRY_INDEX)),
                "the `[patch]` section cannot specify both `{DEFAULT_REGISTRY_INDEX_PATCH_SOURCE}` and `{DEFAULT_REGISTRY_INDEX}`"
            );
            patch
                .into_iter()
                .map(|(source, patches)| {
                    let source = if source == default_index_patch_source {
                        SourceId::default().canonical_url.clone()
                    } else {
                        let url = Url::parse(source.as_str()).with_context(|| {
                            format!("failed to parse `{}` as patch source url", source.as_str())
                        })?;
                        CanonicalUrl::new(&url)?
                    };
                    Ok((
                        source,
                        patches
                            .into_iter()
                            .map(|(name, dep)| {
                                dep.resolve().to_dependency(
                                    name.clone(),
                                    manifest_path,
                                    DepKind::Normal,
                                )
                            })
                            .collect::<Result<Vec<ManifestDependency>>>()?,
                    ))
                })
                .collect()
        } else {
            Ok(BTreeMap::new())
        }
    }
}

fn merge_profile(target: &TomlProfile, source: &TomlProfile) -> Result<TomlProfile> {
    let inherits = source.inherits.clone().or(target.inherits.clone());
    let cairo = if let (Some(target), Some(source)) = (&target.cairo, &source.cairo) {
        Some(toml_merge(target, source)?.clone())
    } else {
        source.cairo.clone().or(target.cairo.clone())
    };
    let tool = if let (Some(target), Some(source)) = (&target.tool, &source.tool) {
        Some(toml_merge(target, source)?.clone())
    } else {
        source.tool.clone().or(target.tool.clone())
    };
    Ok(TomlProfile {
        inherits,
        cairo,
        tool,
    })
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

    file_name
        .map(|file_name| abs_canonical_path("readme", package_root, file_name))
        .transpose()
}

/// Creates the absolute canonical path of the file and checks if it exists
fn abs_canonical_path(file_label: &str, prefix: &Utf8Path, path: &Utf8Path) -> Result<Utf8PathBuf> {
    let path = prefix.parent().unwrap().join(path);
    let path = fsx::canonicalize_utf8(&path)
        .with_context(|| format!("failed to find {file_label} at {path}"))?;
    Ok(path)
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
        dep_kind: DepKind,
    ) -> Result<ManifestDependency> {
        self.resolve().to_dependency(name, manifest_path, dep_kind)
    }
}

impl DetailedTomlDependency {
    fn to_dependency(
        &self,
        name: PackageName,
        manifest_path: &Utf8Path,
        dep_kind: DepKind,
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
        let source_id = match (
            self.version.as_ref(),
            self.git.as_ref(),
            self.path.as_ref(),
            self.registry.as_ref(),
        ) {
            (None, None, None, _) => bail!(
                "dependency ({name}) must be specified providing a local path, Git repository, \
                or version to use"
            ),

            (_, Some(_), Some(_), _) => bail!(
                "dependency ({name}) specification is ambiguous, \
                only one of `git` or `path` is allowed"
            ),

            (_, Some(_), _, Some(_)) => bail!(
                "dependency ({name}) specification is ambiguous, \
                only one of `git` or `registry` is allowed"
            ),

            (_, None, Some(path), _) => {
                let path = path
                    .relative_to_file(manifest_path)?
                    .join(MANIFEST_FILE_NAME);
                SourceId::for_path(&path)?
            }

            (_, Some(git), None, None) => {
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

            (Some(_), None, None, Some(url)) => SourceId::for_registry(url)?,
            (Some(_), None, None, None) => SourceId::default(),
        };

        let features = self.features.clone().unwrap_or_default();
        let features = features
            .into_iter()
            .map(FeatureName::try_new)
            .collect::<Result<Vec<_>>>()?;
        let default_features = self.default_features.unwrap_or(true);

        Ok(ManifestDependency::builder()
            .name(name)
            .source_id(source_id)
            .version_req(version_req)
            .kind(dep_kind)
            .features(features)
            .default_features(default_features)
            .build())
    }
}
