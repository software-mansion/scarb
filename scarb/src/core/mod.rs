//! Core datastructures describing Scarb workspace state.
//!
//! For read operations and workspace mutations, see [`crate::ops`] module.

pub use config::Config;
pub use manifest::{
    DetailedTomlDependency, Manifest, ManifestDependency, Summary, SummaryInner, TomlDependency,
    TomlManifest, TomlPackage,
};
pub use package::{Package, PackageId, PackageIdInner, PackageInner};
pub use resolver::{CompilationUnit, Resolve};
pub use source::{GitReference, SourceId, SourceIdInner, SourceKind};
pub use workspace::Workspace;

pub use crate::DEFAULT_SOURCE_DIR_NAME;
pub use crate::DEFAULT_TARGET_DIR_NAME;
pub use crate::MANIFEST_FILE_NAME;

pub(crate) mod config;
pub(crate) mod manifest;
pub(crate) mod package;
pub(crate) mod registry;
pub(crate) mod resolver;
pub(crate) mod restricted_names;
pub(crate) mod source;
pub(crate) mod workspace;
