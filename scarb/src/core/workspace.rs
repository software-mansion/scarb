use std::fmt;

use anyhow::Result;
use camino::Utf8Path;

use crate::core::config::Config;
use crate::core::package::Package;
use crate::flock::RootFilesystem;
use crate::MANIFEST_FILE_NAME;

// TODO(mkaput): Support real workspaces.
/// The core abstraction for working with a workspace of packages.
///
/// **Note:** Currently only single-package workspaces are supported.
///
/// A workspace is often created very early on and then threaded through all other functions.
/// It's typically through this object that the current package is loaded and/or learned about.
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

    pub fn root(&self) -> &Utf8Path {
        self.package.root()
    }

    pub fn manifest_path(&self) -> &Utf8Path {
        self.package.manifest_path()
    }

    pub fn target_dir(&self) -> &RootFilesystem {
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

impl<'c> fmt::Display for Workspace<'c> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let manifest_path = self.manifest_path();
        let path = if manifest_path.file_name() == Some(MANIFEST_FILE_NAME) {
            self.root()
        } else {
            manifest_path
        };
        write!(f, "{path}")
    }
}

impl<'c> fmt::Debug for Workspace<'c> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Workspace")
            .field("package", &self.package)
            .finish_non_exhaustive()
    }
}
