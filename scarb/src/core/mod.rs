//! Core datastructures describing Scarb workspace state.
//!
//! For read operations and workspace mutations, see [`crate::ops`] module.

pub use config::Config;
pub use dirs::AppDirs;
pub use manifest::{
    DetailedTomlDependency, ExternalTargetKind, LibTargetKind, Manifest, ManifestDependency,
    ManifestMetadata, Summary, SummaryInner, Target, TargetInner, TargetKind, TomlDependency,
    TomlExternalTarget, TomlLibTarget, TomlManifest, TomlPackage, TomlTargetKindName,
};
pub use package::{Package, PackageId, PackageIdInner, PackageInner, PackageName};
pub use resolver::{PackageComponentsIds, Resolve};
pub use source::{GitReference, SourceId, SourceIdInner, SourceKind};
pub use workspace::Workspace;

pub use crate::DEFAULT_SOURCE_DIR_NAME;
pub use crate::DEFAULT_TARGET_DIR_NAME;
pub use crate::MANIFEST_FILE_NAME;

pub(crate) mod config;
mod dirs;
pub(crate) mod manifest;
pub(crate) mod package;
pub(crate) mod registry;
pub(crate) mod resolver;
pub(crate) mod source;
pub(crate) mod workspace;
