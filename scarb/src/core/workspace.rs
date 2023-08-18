use std::collections::BTreeMap;
use std::fmt;

use anyhow::{anyhow, bail, Result};
use camino::{Utf8Path, Utf8PathBuf};
use scarb_ui::args::PackagesSource;

use crate::compiler::Profile;
use crate::core::config::Config;
use crate::core::package::Package;
use crate::core::PackageId;
use crate::flock::RootFilesystem;
use crate::MANIFEST_FILE_NAME;

/// The core abstraction for working with a workspace of packages.
///
/// A workspace is often created very early on and then threaded through all other functions.
/// It's typically through this object that the current package is loaded and/or learned about.
pub struct Workspace<'c> {
    config: &'c Config,
    members: BTreeMap<PackageId, Package>,
    root_package: Option<PackageId>,
    manifest_path: Utf8PathBuf,
    profiles: Vec<Profile>,
}

impl<'c> Workspace<'c> {
    pub(crate) fn new(
        manifest_path: Utf8PathBuf,
        packages: &[Package],
        root_package: Option<PackageId>,
        config: &'c Config,
        profiles: Vec<Profile>,
    ) -> Result<Self> {
        let packages = packages
            .iter()
            .map(|p| (p.id, p.clone()))
            .collect::<BTreeMap<_, _>>();
        Ok(Self {
            config,
            manifest_path,
            root_package,
            profiles,
            members: packages,
        })
    }

    pub(crate) fn from_single_package(
        package: Package,
        config: &'c Config,
        profiles: Vec<Profile>,
    ) -> Result<Self> {
        let manifest_path = package.manifest_path().to_path_buf();
        let root_package = Some(package.id);
        Self::new(
            manifest_path,
            vec![package].as_ref(),
            root_package,
            config,
            profiles,
        )
    }

    /// Returns the [`Config`] this workspace is associated with.
    pub fn config(&self) -> &'c Config {
        self.config
    }

    pub fn root(&self) -> &Utf8Path {
        self.manifest_path
            .parent()
            .expect("manifest path parent must always exist")
    }

    pub fn manifest_path(&self) -> &Utf8Path {
        &self.manifest_path
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
        self.members
            .values()
            .find(|p| p.manifest_path() == self.config.manifest_path())
            .ok_or_else(|| {
                anyhow!("could not determine which package to use, please specify a package with the `--package` option")
            })
    }

    /// Returns the root package of this workspace.
    ///
    /// Root package is defined by `[package]` section in workspace manifest file.
    /// If workspace manifest file does not contain `[package]` section,
    /// that is there is no Scarb manifest with both `[package]` and `[workspace]` sections,
    /// then there is no root package.
    pub fn root_package(&self) -> Option<Package> {
        self.root_package.and_then(|id| self.package(&id)).cloned()
    }

    pub fn package(&self, id: &PackageId) -> Option<&Package> {
        self.members.get(id)
    }

    pub fn fetch_package(&self, id: &PackageId) -> Result<&Package> {
        self.package(id)
            .ok_or_else(|| anyhow!("package `{}` is not a member of this workspace", id))
    }

    /// Returns an iterator over all packages in this workspace
    pub fn members(&self) -> impl Iterator<Item = Package> + '_ {
        self.members.values().cloned()
    }

    /// Return members count
    pub fn members_count(&self) -> usize {
        self.members.len()
    }

    /// Return whether the workspace has exactly one package
    pub fn is_single_package(&self) -> bool {
        self.members_count() == 1
    }

    pub fn has_profile(&self, profile: &Profile) -> bool {
        self.profiles.contains(profile)
    }

    pub fn current_profile(&self) -> Result<Profile> {
        let profile = self.config.profile();
        if profile.is_custom() && !self.has_profile(&profile) {
            bail!("workspace `{self}` has no profile `{profile}`",);
        }
        Ok(profile)
    }

    pub fn profile_names(&self) -> Result<Vec<String>> {
        let mut names = self
            .profiles
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>();
        names.push(Profile::DEV.to_string());
        names.push(Profile::RELEASE.to_string());
        names.sort();
        Ok(names)
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
            .field("members", &self.members)
            .finish_non_exhaustive()
    }
}

pub trait Utf8PathWorkspaceExt {
    fn workspace_relative(&self, ws: &Workspace<'_>) -> &Utf8Path;
}

impl Utf8PathWorkspaceExt for Utf8Path {
    fn workspace_relative(&self, ws: &Workspace<'_>) -> &Utf8Path {
        self.strip_prefix(ws.root()).unwrap_or(self)
    }
}

impl<'c> PackagesSource for Workspace<'c> {
    type Package = Package;

    fn package_name_of(package: &Self::Package) -> &str {
        package.id.name.as_str()
    }

    fn members(&self) -> Vec<Self::Package> {
        Workspace::members(self).collect()
    }

    fn runtime_manifest(&self) -> Utf8PathBuf {
        self.config.manifest_path().to_path_buf()
    }
}
