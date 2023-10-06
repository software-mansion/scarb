//! Core datastructures describing Scarb workspace state.
//!
//! For read operations and workspace mutations, see [`crate::ops`] module.

pub use config::Config;
pub use dirs::AppDirs;
pub use manifest::*;
pub use package::{Package, PackageId, PackageIdInner, PackageInner, PackageName};
pub use resolver::Resolve;
pub use source::{GitReference, SourceId, SourceIdInner, SourceKind};
pub use workspace::{Utf8PathWorkspaceExt, Workspace};

pub(crate) mod config;
mod dirs;
pub mod errors;
pub(crate) mod manifest;
pub(crate) mod package;
pub(crate) mod publishing;
pub(crate) mod registry;
pub(crate) mod resolver;
pub(crate) mod source;
pub(crate) mod workspace;
