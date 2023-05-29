#![deny(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![warn(rust_2018_idioms)]
#![doc = concat!(
    "Structured access to the output of `scarb metadata --format-version ",
    env!("CARGO_PKG_VERSION_MAJOR"),
    "`.
")]
//! Usually used by Scarb extensions and other developer tools.
//!
//! [Scarb](https://docs.swmansion.com/scarb) is a build toolchain and package manager for
//! the [Cairo language](https://www.cairo-lang.org/).
//! See the [Scarb documentation](https://docs.swmansion.com/scarb/docs) for details on
//! Scarb itself.
//!
//! With the `command` feature (enabled by default), this crate also exposes an ergonomic interface
//! to collect metadata from Scarb: [`MetadataCommand`].
//!
//! With the `packages_filter` feature (disabled by default), this crate provides ready to use
//! arguments definitions for the `clap` crate that implement Scarb-compatible package selection
//! (i.e. the `-p/--package` argument).

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::ops::Index;
use std::path::PathBuf;

use camino::{Utf8Path, Utf8PathBuf};
#[cfg(feature = "builder")]
use derive_builder::Builder;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

#[cfg(feature = "command")]
pub use command::*;
pub use version_pin::*;

#[cfg(feature = "command")]
mod command;
#[cfg(feature = "packages_filter")]
pub mod packages_filter;
mod version_pin;

/// An "opaque" identifier for a package.
/// It is possible to inspect the `repr` field, if the need arises,
/// but its precise format is an implementation detail and is subject to change.
///
/// [`Metadata`] can be indexed by [`PackageId`].
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct PackageId {
    /// The underlying string representation of the ID.
    pub repr: String,
}

impl From<String> for PackageId {
    fn from(repr: String) -> Self {
        Self { repr }
    }
}

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.repr, f)
    }
}

/// An "opaque" identifier for a source.
/// It is possible to inspect the `repr` field, if the need arises,
/// but its precise format is an implementation detail and is subject to change.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct SourceId {
    /// The underlying string representation of the ID.
    pub repr: String,
}

impl From<String> for SourceId {
    fn from(repr: String) -> Self {
        Self { repr }
    }
}

impl fmt::Display for SourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.repr, f)
    }
}

/// An "opaque" identifier for a compilation unit.
/// It is possible to inspect the `repr` field, if the need arises,
/// but its precise format is an implementation detail and is subject to change.
///
/// [`Metadata`] can be indexed by [`CompilationUnitId`].
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct CompilationUnitId {
    /// The underlying string representation of the ID.
    pub repr: String,
}

impl From<String> for CompilationUnitId {
    fn from(repr: String) -> Self {
        Self { repr }
    }
}

impl fmt::Display for CompilationUnitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.repr, f)
    }
}

fn current_profile_default() -> String {
    "release".to_string()
}
fn profiles_default() -> Vec<String> {
    vec!["release".to_string()]
}

/// Top level data structure printed by `scarb metadata`.
#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "builder", derive(Builder))]
#[cfg_attr(feature = "builder", builder(setter(into)))]
#[non_exhaustive]
pub struct Metadata {
    // NOTE: This field must always be first! `MetadataCommand` is assuming this.
    /// The metadata format version.
    ///
    /// This struct will not deserialize if version does not match.
    #[cfg_attr(feature = "builder", builder(setter(skip)))]
    pub version: VersionPin,

    /// Path to `scarb` executable.
    pub app_exe: Option<PathBuf>,

    /// Scarb's version.
    pub app_version_info: VersionInfo,

    /// Path to the _target_ (_build_) directory if known by Scarb at the moment of generating
    /// metadata.
    pub target_dir: Option<Utf8PathBuf>,

    /// Current workspace metadata.
    pub workspace: WorkspaceMetadata,

    /// Metadata of all packages used in this workspace, or just members of it if this is an output
    /// of `scarb metadata --no-deps`.
    ///
    /// In the former case, use [`WorkspaceMetadata::members`] to filter workspace members.
    pub packages: Vec<PackageMetadata>,

    /// List of all Scarb compilation units produced in this workspace.
    pub compilation_units: Vec<CompilationUnitMetadata>,

    /// Name of the currently selected profile
    #[serde(default = "current_profile_default")]
    pub current_profile: String,

    /// List of all available profiles names
    #[serde(default = "profiles_default")]
    pub profiles: Vec<String>,

    /// Additional data not captured by deserializer.
    #[cfg_attr(feature = "builder", builder(default))]
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Current workspace metadata.
#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "builder", derive(Builder))]
#[cfg_attr(feature = "builder", builder(setter(into)))]
#[non_exhaustive]
pub struct WorkspaceMetadata {
    /// Path to the manifest file defining this workspace.
    pub manifest_path: Utf8PathBuf,

    /// Path to the directory containing this workspace.
    pub root: Utf8PathBuf,

    /// List of IDs of all packages that are members of this workspace.
    pub members: Vec<PackageId>,

    /// Additional data not captured by deserializer.
    #[cfg_attr(feature = "builder", builder(default))]
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Metadata of single Scarb package.
#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "builder", derive(Builder))]
#[cfg_attr(feature = "builder", builder(setter(into)))]
#[non_exhaustive]
pub struct PackageMetadata {
    /// Package ID.
    pub id: PackageId,

    /// Package name as given in `Scarb.toml`.
    pub name: String,

    /// Package version as given in `Scarb.toml`.
    pub version: Version,

    /// The source of the package.
    pub source: SourceId,

    /// Path to the manifest file defining this package.
    pub manifest_path: Utf8PathBuf,

    /// Path to the directory containing this package.
    pub root: Utf8PathBuf,

    /// List of dependencies of this particular package.
    pub dependencies: Vec<DependencyMetadata>,

    /// Targets provided by the package. (`lib`, `starknet-contract`, etc.).
    pub targets: Vec<TargetMetadata>,

    /// Various metadata fields from `Scarb.toml`.
    #[serde(flatten)]
    pub manifest_metadata: ManifestMetadata,

    /// Additional data not captured by deserializer.
    #[cfg_attr(feature = "builder", builder(default))]
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Scarb package dependency specification.
///
/// Only the `name` field is strictly sourced from `Scarb.toml`, the rest is processed by Scarb
/// when processing this file.
#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "builder", derive(Builder))]
#[cfg_attr(feature = "builder", builder(setter(into)))]
#[non_exhaustive]
pub struct DependencyMetadata {
    /// Package name.
    pub name: String,
    /// Package version requirement.
    pub version_req: VersionReq,
    /// Package source.
    pub source: SourceId,

    /// Additional data not captured by deserializer.
    #[cfg_attr(feature = "builder", builder(default))]
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Package target information.
#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "builder", derive(Builder))]
#[cfg_attr(feature = "builder", builder(setter(into)))]
#[non_exhaustive]
pub struct TargetMetadata {
    /// Target kind: `lib`, `starknet-contract`, etc.
    pub kind: String,
    /// Target name, often this is a default, which is the package name.
    pub name: String,
    /// Path to the main source file of the target.
    pub source_path: Utf8PathBuf,
    /// Unstructured target parameters, excluding default values.
    ///
    /// Default values are omitted because they are unknown to Scarb, they are applied by compilers.
    pub params: serde_json::Value,

    /// Additional data not captured by deserializer.
    #[cfg_attr(feature = "builder", builder(default))]
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Scarb compilation unit information.
#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "builder", derive(Builder))]
#[cfg_attr(feature = "builder", builder(setter(into)))]
#[non_exhaustive]
pub struct CompilationUnitMetadata {
    /// Unique ID of this compilation unit.
    pub id: CompilationUnitId,

    /// Main package to be built.
    pub package: PackageId,

    /// Selected target of the main package.
    pub target: TargetMetadata,

    /// Cairo compiler config.
    ///
    /// This is unstructured, because this can rapidly change throughout Scarb lifetime.
    pub compiler_config: serde_json::Value,

    // TODO(mkaput): Perhaps rename this back to `components` in Scarb >=0.3?
    /// List of all components to include in this compilation.
    #[serde(rename = "components_data")]
    pub components: Vec<CompilationUnitComponentMetadata>,

    /// List of all Cairo compiler plugins to load in this compilation.
    #[serde(default)]
    pub cairo_plugins: Vec<CompilationUnitCairoPluginMetadata>,

    /// Items for the Cairo's `#[cfg(...)]` attribute to be enabled in this unit.
    #[serde(default)]
    pub cfg: Vec<Cfg>,

    /// Additional data not captured by deserializer.
    #[cfg_attr(feature = "builder", builder(default))]
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Information to pass to the Cairo compiler about a package that is a component of a compilation
/// unit.
///
/// List of components can be used to construct the `[crate_roots]` section of `cairo_project.toml`.
#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "builder", derive(Builder))]
#[cfg_attr(feature = "builder", builder(setter(into)))]
#[non_exhaustive]
pub struct CompilationUnitComponentMetadata {
    /// Package ID.
    pub package: PackageId,
    /// Name of the package to pass to the Cairo compiler.
    ///
    /// This may not be equal to Scarb package name in the future.
    pub name: String,
    /// Path to the root Cairo source file.
    pub source_path: Utf8PathBuf,

    /// Additional data not captured by deserializer.
    #[cfg_attr(feature = "builder", builder(default))]
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Information about compiler plugin to load into the Cairo compiler as part of a compilation unit.
#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "builder", derive(Builder))]
#[cfg_attr(feature = "builder", builder(setter(into)))]
#[non_exhaustive]
pub struct CompilationUnitCairoPluginMetadata {
    /// Package ID.
    pub package: PackageId,

    /// Additional data not captured by deserializer.
    #[cfg_attr(feature = "builder", builder(default))]
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Various metadata fields from package manifest.
#[derive(Clone, Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "builder", derive(Builder))]
#[cfg_attr(feature = "builder", builder(setter(into)))]
#[non_exhaustive]
pub struct ManifestMetadata {
    /// List of the people or organizations that are considered the "authors" of the package.
    pub authors: Option<Vec<String>>,
    /// A short blurb about the package.
    pub description: Option<String>,
    /// A URL to a website hosting the crate's documentation.
    pub documentation: Option<String>,
    /// A URL to a site that is the home page for this package.
    pub homepage: Option<String>,
    /// An array of strings that describe this package.
    pub keywords: Option<Vec<String>>,
    /// Name of the software license that the package is released under.
    ///
    /// Should be an [SPDX 2 license expression(opens in a new tab)](https://spdx.github.io/spdx-spec/v2.3/SPDX-license-expressions/),
    /// but this is not validated neither by this crate nor Scarb.
    pub license: Option<String>,
    /// A path to a file containing the text of package's license (relative to its `Scarb.toml`).
    pub license_file: Option<String>,
    /// A path to a file in the package root (relative to its `Scarb.toml`) that contains general
    /// information about the package.
    pub readme: Option<String>,
    /// A URL to the source repository for this package.
    pub repository: Option<String>,
    /// A map of additional internet links related to this package.
    pub urls: Option<BTreeMap<String, String>>,
    /// Various unstructured metadata to be used by external tools.
    pub tool: Option<BTreeMap<String, serde_json::Value>>,
}

/// Scarb's version.
#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "builder", derive(Builder))]
#[cfg_attr(feature = "builder", builder(setter(into)))]
#[non_exhaustive]
pub struct VersionInfo {
    /// Version of Scarb.
    pub version: Version,
    /// Version about Git commit of Scarb if known.
    pub commit_info: Option<CommitInfo>,
    /// Version of the Cairo compiler bundled in Scarb.
    pub cairo: CairoVersionInfo,

    /// Additional data not captured by deserializer.
    #[cfg_attr(feature = "builder", builder(default))]
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Cairo's version.
#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "builder", derive(Builder))]
#[cfg_attr(feature = "builder", builder(setter(into)))]
#[non_exhaustive]
pub struct CairoVersionInfo {
    /// Version of the Cairo compiler.
    pub version: Version,
    /// Version about Git commit of Cairo if known.
    pub commit_info: Option<CommitInfo>,

    /// Additional data not captured by deserializer.
    #[cfg_attr(feature = "builder", builder(default))]
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Information about the Git repository where Scarb or Cairo was built from.
#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "builder", derive(Builder))]
#[cfg_attr(feature = "builder", builder(setter(into)))]
#[non_exhaustive]
pub struct CommitInfo {
    /// Git commit hash, shortened.
    pub short_commit_hash: String,
    /// Git commit hash.
    pub commit_hash: String,
    /// Commit author date if known.
    pub commit_date: Option<String>,
}

/// Option for the `#[cfg(...)]` language attribute.
#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum Cfg {
    /// `#[cfg(key: value)`]
    KV(String, String),
    /// `#[cfg(name)`]
    Name(String),
}

impl Metadata {
    /// Returns reference to [`PackageMetadata`] corresponding to the [`PackageId`].
    pub fn get_package(&self, id: &PackageId) -> Option<&PackageMetadata> {
        self.packages.iter().find(|p| p.id == *id)
    }

    /// Returns reference to [`CompilationUnitMetadata`] corresponding to the [`CompilationUnitId`].
    pub fn get_compilation_unit(&self, id: &CompilationUnitId) -> Option<&CompilationUnitMetadata> {
        self.compilation_units.iter().find(|p| p.id == *id)
    }
}

impl<'a> Index<&'a PackageId> for Metadata {
    type Output = PackageMetadata;

    fn index(&self, idx: &'a PackageId) -> &Self::Output {
        self.get_package(idx)
            .unwrap_or_else(|| panic!("no package with this ID: {idx}"))
    }
}

impl<'a> Index<&'a CompilationUnitId> for Metadata {
    type Output = CompilationUnitMetadata;

    fn index(&self, idx: &'a CompilationUnitId) -> &Self::Output {
        self.get_compilation_unit(idx)
            .unwrap_or_else(|| panic!("no compilation unit with this ID: {idx}"))
    }
}

impl PackageMetadata {
    /// Get value of the `[tool.*]` section in this package's manifest, for specific `tool_name`,
    /// including any transformations applied by Scarb.
    pub fn tool_metadata(&self, tool_name: &str) -> Option<&serde_json::Value> {
        self.manifest_metadata.tool.as_ref()?.get(tool_name)
    }
}

impl TargetMetadata {
    /// Path to the main source directory of the target.
    pub fn source_root(&self) -> &Utf8Path {
        self.source_path
            .parent()
            .expect("Source path is guaranteed to point to a file.")
    }
}

impl CompilationUnitComponentMetadata {
    /// Path to the source directory of the component.
    pub fn source_root(&self) -> &Utf8Path {
        self.source_path
            .parent()
            .expect("Source path is guaranteed to point to a file.")
    }
}
