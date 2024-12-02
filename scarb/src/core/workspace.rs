use std::collections::{BTreeMap, HashSet};
use std::fmt;

use anyhow::{anyhow, bail, Result};
use camino::{Utf8Path, Utf8PathBuf};
use itertools::Itertools;
use scarb_ui::args::PackagesSource;
use smol_str::SmolStr;

use crate::compiler::Profile;
use crate::core::config::Config;
use crate::core::package::Package;
use crate::core::{PackageId, ScriptDefinition, Target};
use crate::flock::Filesystem;
use crate::{DEFAULT_TARGET_DIR_NAME, LOCK_FILE_NAME, MANIFEST_FILE_NAME};

/// The core abstraction for working with a workspace of packages.
///
/// A workspace is often created very early on and then threaded through all other functions.
/// It's typically through this object that the current package is loaded and/or learned about.
pub struct Workspace<'c> {
    config: &'c Config,
    members: BTreeMap<PackageId, Package>,
    manifest_path: Utf8PathBuf,
    profiles: Vec<Profile>,
    scripts: BTreeMap<SmolStr, ScriptDefinition>,
    root_package: Option<PackageId>,
    target_dir: Filesystem,
}

impl<'c> Workspace<'c> {
    pub(crate) fn new(
        manifest_path: Utf8PathBuf,
        packages: &[Package],
        root_package: Option<PackageId>,
        config: &'c Config,
        profiles: Vec<Profile>,
        scripts: BTreeMap<SmolStr, ScriptDefinition>,
    ) -> Result<Self> {
        let targets = packages
            .iter()
            .flat_map(|p| p.manifest.targets.iter())
            .collect_vec();
        check_unique_targets(&targets)?;

        let packages = packages
            .iter()
            .map(|p| (p.id, p.clone()))
            .collect::<BTreeMap<_, _>>();
        let target_dir = config.target_dir_override().cloned().unwrap_or_else(|| {
            manifest_path
                .parent()
                .expect("parent of manifest path must always exist")
                .join(DEFAULT_TARGET_DIR_NAME)
        });
        let target_dir = Filesystem::new_output_dir(target_dir);
        Ok(Self {
            config,
            manifest_path,
            profiles,
            root_package,
            target_dir,
            members: packages,
            scripts,
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
            BTreeMap::new(),
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

    pub fn lockfile_path(&self) -> Utf8PathBuf {
        self.root().join(LOCK_FILE_NAME)
    }

    pub fn target_dir(&self) -> &Filesystem {
        &self.target_dir
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

    pub fn profile_names(&self) -> Vec<String> {
        let mut names = self
            .profiles
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>();
        names.push(Profile::DEV.to_string());
        names.push(Profile::RELEASE.to_string());
        names.sort();
        names.dedup();
        names
    }

    pub fn scripts(&self) -> &BTreeMap<SmolStr, ScriptDefinition> {
        &self.scripts
    }

    pub fn script(&self, name: &SmolStr) -> Option<&ScriptDefinition> {
        self.scripts.get(name)
    }
}

fn check_unique_targets(targets: &Vec<&Target>) -> Result<()> {
    let mut used = HashSet::with_capacity(targets.len());
    for target in targets {
        if !used.insert((target.kind.as_str(), target.name.as_str())) {
            bail!(
                "workspace contains duplicate target definitions `{} ({})`\n\
                 help: use different target names to resolve the conflict",
                target.kind,
                target.name
            )
        }
    }
    for (kind, group_id) in targets
        .iter()
        .filter_map(|target| {
            target
                .group_id
                .clone()
                .map(|group_id| (target.kind.clone(), group_id))
        })
        .unique()
    {
        if used.contains(&(kind.as_str(), group_id.as_str())) {
            bail!(
                "the group id `{group_id}` of target `{kind}` duplicates target name\n\
                 help: use different group name to resolve the conflict",
            )
        }
    }
    Ok(())
}

impl fmt::Display for Workspace<'_> {
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

impl fmt::Debug for Workspace<'_> {
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

impl PackagesSource for Workspace<'_> {
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
