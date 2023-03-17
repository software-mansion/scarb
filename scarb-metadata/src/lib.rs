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

use std::collections::BTreeMap;
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
mod version_pin;

/// An "opaque" identifier for a package.
/// It is possible to inspect the `repr` field, if the need arises,
/// but its precise format is an implementation detail and is subject to change.
///
/// `Metadata` can be indexed by `PackageId`.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct PackageId {
    /// The underlying string representation of the ID.
    pub repr: String,
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

impl fmt::Display for SourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.repr, f)
    }
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
    #[builder(setter(skip))]
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
}

/// Current workspace metadata.
#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "builder", derive(Builder))]
#[cfg_attr(feature = "builder", builder(setter(into)))]
#[non_exhaustive]
pub struct WorkspaceMetadata {
    /// Path to the manifest file defining this workspace.
    pub manifest_path: Utf8PathBuf,

    /// List of IDs of all packages that are members of this workspace.
    pub members: Vec<PackageId>,
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

    /// List of dependencies of this particular package.
    pub dependencies: Vec<DependencyMetadata>,

    /// Targets provided by the package. (`lib`, `starknet-contract`, etc.).
    pub targets: Vec<TargetMetadata>,

    /// Various metadata fields from `Scarb.toml`.
    #[serde(flatten)]
    pub manifest_metadata: ManifestMetadata,
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
    /// Unstructured target parameters.
    pub params: serde_json::Value,
}

/// Scarb compilation unit information.
#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "builder", derive(Builder))]
#[cfg_attr(feature = "builder", builder(setter(into)))]
#[non_exhaustive]
pub struct CompilationUnitMetadata {
    /// Main package to be built.
    pub package: PackageId,
    /// Selected target of the main package.
    pub target: TargetMetadata,
    /// IDs of all packages to be included in this compilation.
    ///
    /// This is the ID of the main package and all its transitive dependencies.
    pub components: Vec<PackageId>,
    /// Cairo compiler config.
    ///
    /// This is unstructured, because this can rapidly change throughout Scarb lifetime.
    pub compiler_config: serde_json::Value,
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

impl<'a> Index<&'a PackageId> for Metadata {
    type Output = PackageMetadata;

    fn index(&self, idx: &'a PackageId) -> &Self::Output {
        self.packages
            .iter()
            .find(|p| p.id == *idx)
            .unwrap_or_else(|| panic!("no package with this ID: {idx}"))
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
