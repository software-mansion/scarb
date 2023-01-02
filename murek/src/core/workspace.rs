use std::path::Path;

use anyhow::Result;

use crate::core::config::{Config, TargetDir};
use crate::core::package::Package;

// TODO(mkaput): Support real workspaces.
/// The core abstraction for working with a workspace of packages.
///
/// **Note:** Currently only single-package workspaces are supported.
///
/// A workspace is often created very early on and then threaded through all other functions.
/// It's typically through this object that the current package is loaded and/or learned about.
#[derive(Debug)]
pub struct Workspace<'c> {
    config: &'c Config,
    package: Package,
}

impl<'c> Workspace<'c> {
    pub(crate) fn from_single_package(package: Package, config: &'c Config) -> Result<Self> {
        Ok(Self { config, package })
    }

    /// Returns the [`Config`] this workspace is associated with.
    pub fn config(&self) -> &'c Config {
        self.config
    }

    pub fn root(&self) -> &Path {
        self.package.root()
    }

    pub fn manifest_path(&self) -> &Path {
        self.package.manifest_path()
    }

    pub fn target_dir(&self) -> Result<&TargetDir> {
        self.config.target_dir()
    }

    /// Returns the current package of this workspace.
    ///
    /// Note that this can return an error in the future,
    /// when workspace-specific manifests will be implemented.
    /// In this case an error is returned indicating that the operation
    /// must be performed on specific package.
    pub fn current_package(&self) -> Result<&Package> {
        Ok(&self.package)
    }

    /// Returns an iterator over all packages in this workspace
    pub fn members(&self) -> impl Iterator<Item = Package> {
        [self.package.clone()].into_iter()
    }
}
