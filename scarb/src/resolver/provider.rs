use crate::core::lockfile::{Lockfile, PackageLock};
use crate::core::registry::patch_map::PatchMap;
use crate::core::{
    DepKind, DependencyFilter, DependencyVersionReq, ManifestDependency, PackageId, PackageName,
    SourceId, SourceKind, Summary,
};
use crate::resolver::in_memory_index::VersionsResponse;
use crate::resolver::{Request, ResolverState};
use itertools::Itertools;
use pubgrub::{Dependencies, DependencyProvider, Range};
use pubgrub::{Ranges, VersionSet};
use semver::{Version, VersionReq};
use semver_pubgrub::SemverPubgrub;
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct CustomIncompatibility(String);

impl Display for CustomIncompatibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Package identifier for PubGrub algorithm.
/// This identifier is stripped from version, which is represented with [`semver::Version`] instead.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PubGrubPackage {
    pub name: PackageName,
    pub source_id: SourceId,
}

impl PubGrubPackage {
    fn to_dependency(&self, range: SemverPubgrub) -> ManifestDependency {
        ManifestDependency::builder()
            .name(self.name.clone())
            .source_id(self.source_id)
            .version_req(range.into())
            .build()
    }
}

impl From<SemverPubgrub> for DependencyVersionReq {
    /// This conversion will always return a [`DependencyVersionReq`] that includes the provided
    /// range. It's not guaranteed that the range will be exactly the same as the original one.
    /// It will never be more restrictive than the original range though.
    fn from(range: SemverPubgrub) -> Self {
        let Some((start, end)) = range.bounding_range() else {
            return DependencyVersionReq::Any;
        };
        let bounds = (start.map(|b| b.clone()), end.map(|b| b.clone()));
        let range: Ranges<Version> = Range::from_range_bounds(bounds);
        VersionReq::parse(&range.to_string())
            .map(|req| {
                if req.comparators.is_empty() {
                    return DependencyVersionReq::Any;
                }
                DependencyVersionReq::Req(req)
            })
            .unwrap_or(DependencyVersionReq::Any)
    }
}

impl From<&ManifestDependency> for PubGrubPackage {
    fn from(dependency: &ManifestDependency) -> Self {
        Self {
            name: dependency.name.clone(),
            source_id: dependency.source_id,
        }
    }
}

impl From<PackageId> for PubGrubPackage {
    fn from(package_id: PackageId) -> Self {
        Self {
            name: package_id.name.clone(),
            source_id: package_id.source_id,
        }
    }
}

impl From<&Summary> for PubGrubPackage {
    fn from(summary: &Summary) -> Self {
        summary.package_id.into()
    }
}

impl Display for PubGrubPackage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(dead_code)]
pub enum PubGrubPriority {
    /// The package has no specific priority.
    ///
    /// As such, its priority is based on the order in which the packages were added (FIFO), such
    /// that the first package we visit is prioritized over subsequent packages.
    Unspecified(Reverse<usize>),

    /// The version range is constrained to a single version (e.g., with the `==` operator).
    Singleton(Reverse<usize>),

    /// The package was specified via a direct URL.
    DirectUrl(Reverse<usize>),

    /// The package is the root package.
    Root,
}

pub struct PubGrubDependencyProvider {
    priority: RwLock<HashMap<PubGrubPackage, usize>>,
    packages: RwLock<HashMap<PackageId, Summary>>,
    kinds: RwLock<HashMap<PubGrubPackage, DepKind>>,
    main_package_ids: HashSet<PackageId>,
    patch_map: PatchMap,
    lockfile: Lockfile,
    state: Arc<ResolverState>,
    request_sink: mpsc::Sender<Request>,
    yanked_whitelist: HashSet<PackageId>,
    require_audits: bool,
    require_audits_whitelist: HashSet<PackageName>,
}

impl PubGrubDependencyProvider {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        main_package_ids: HashSet<PackageId>,
        state: Arc<ResolverState>,
        request_sink: mpsc::Sender<Request>,
        patch_map: PatchMap,
        lockfile: Lockfile,
        yanked_whitelist: HashSet<PackageId>,
        require_audits: bool,
        require_audits_whitelist: HashSet<PackageName>,
    ) -> Self {
        Self {
            main_package_ids,
            priority: RwLock::new(HashMap::new()),
            packages: RwLock::new(HashMap::new()),
            kinds: RwLock::new(HashMap::new()),
            state,
            patch_map,
            lockfile,
            request_sink,
            yanked_whitelist,
            require_audits,
            require_audits_whitelist,
        }
    }

    /// Ids of packages that we resolve dependency graph for.
    pub fn main_package_ids(&self) -> &HashSet<PackageId> {
        &self.main_package_ids
    }

    /// Request a dependency to be fetched.
    ///
    /// Dependencies are fetched in the background, on another thread, by [`ResolverState`].
    /// This function sends a request to the [`ResolverState`] to fetch the dependency via a channel.
    ///
    /// This function decides whether the dependency should be fetched before sending the request.
    /// Not all dependencies need to be fetched! The algorithm works as follows:
    /// - If the dependency is not from a registry source, it should be fetched.
    ///   Note that this does not necessarily mean they will be fetched from remote sources.
    ///   Fetching a git source may mean only checking out a locally cached repository clone.
    /// - If the dependency comes from a registry source, and its version is locked in the
    ///   `Scarb.lock` file, we can skip fetching the repository from the registry.
    ///   This is safe, since the registry is immutable, and thus if we have previously selected
    ///   a version and locked it in the lockfile, we can safely assume we can download it from the
    ///   registry this time as well. On the contrary, we cannot assume the same about git sources,
    ///   as previously used commit hash may no longer be accessible from a remote repository, so we
    ///   can then only use it if we already have it cloned in our local cache.
    pub fn request_dependency(&self, dependency: ManifestDependency) {
        if (self.require_audits
            || !dependency.source_id.is_registry()
            || !self.lockfile.locks_dependency(dependency.clone()))
            && self
                .state
                .index
                .packages()
                .register(dependency.clone().into())
        {
            self.request_sink
                .blocking_send(Request::Package(dependency.clone()))
                .expect("failed to request dependency download");
        }
    }

    /// Block until a package summary is fetched, then return it.
    ///
    /// This function fetches a single summary, determined by a [`PackageId`].
    /// If you need to fetch summaries by [`ManifestDependency`], use
    /// [`Self::blocking_fetch_summaries_by_dependency`] instead.
    ///
    /// Only use this function if the package summary has been previously requested with
    /// [`Self::request_dependency`] or another method, otherwise this function will panic!
    pub fn blocking_fetch_summary_by_package_id(
        &self,
        package_id: PackageId,
    ) -> Result<Summary, DependencyProviderError> {
        let summary = self
            .packages
            .read()
            .expect("locking resolver state for read failed")
            .get(&package_id)
            .cloned();
        let summary = summary.map(Ok).unwrap_or_else(|| {
            let dependency = ManifestDependency::builder()
                .name(package_id.name.clone())
                .source_id(package_id.source_id)
                .version_req(DependencyVersionReq::exact(&package_id.version))
                .build();
            let summary = self
                .blocking_fetch_summaries_by_dependency(dependency.clone())?
                .into_iter()
                .find_or_first(|summary| summary.package_id == package_id);
            if let Some(summary) = summary.as_ref() {
                let mut write_lock = self
                    .packages
                    .write()
                    .expect("locking resolver state for write failed");
                write_lock.insert(summary.package_id, summary.clone());
                write_lock.insert(package_id, summary.clone());
            }
            summary.ok_or_else(|| DependencyProviderError::PackageNotFound {
                name: dependency.name.clone().to_string(),
                version: dependency.version_req.clone(),
            })
        })?;
        Ok(summary)
    }

    /// Block until a package summary is fetched, then return it.
    ///
    /// This function fetches multiple summaries, determined by a [`ManifestDependency`].
    /// If you need to fetch a single summary identified by [`PackageId`], use
    /// [`Self::blocking_fetch_summary_by_package_id`] instead.
    ///
    /// Only use this function if the package summary has been previously requested with
    /// [`Self::request_dependency`] or another method, otherwise this function will panic!
    #[tracing::instrument(level = "trace", skip(self))]
    fn blocking_fetch_summaries_by_dependency(
        &self,
        dependency: ManifestDependency,
    ) -> Result<Vec<Summary>, DependencyProviderError> {
        let summaries = self
            .state
            .index
            .packages()
            .wait_blocking(&dependency.into())
            .expect("dependency download must start before waiting for it");
        let VersionsResponse::Found(summaries) = summaries.as_ref();

        {
            let mut write_lock = self
                .packages
                .write()
                .expect("locking resolver state for write failed");
            for summary in summaries.iter() {
                write_lock.insert(summary.package_id, summary.clone());
            }
        }

        // Sort from highest to lowest.
        let summaries = summaries
            .iter()
            .sorted_by_key(|sum| sum.package_id.version.clone())
            .rev()
            .cloned()
            .collect_vec();

        Ok(summaries)
    }

    /// Request all dependencies of some package to be fetched.
    ///
    /// This is a shortcut to calling [`Self::request_dependency`] for each of the dependencies.
    fn request_dependencies(&self, summary: &Summary) -> Result<(), DependencyProviderError> {
        for original_dependency in summary.full_dependencies() {
            let original_dependency = self.patch_map.lookup(original_dependency);
            self.save_most_restrictive_kind(&original_dependency);
            let dependency = lock_dependency(&self.lockfile, original_dependency.clone())?;
            self.save_most_restrictive_kind(&dependency);
            self.request_dependency(dependency);

            let dependency =
                rewrite_path_dependency_source_id(summary.package_id, &original_dependency);
            let dependency = lock_dependency(&self.lockfile, dependency)?;
            self.save_most_restrictive_kind(&dependency);
            self.request_dependency(dependency);
        }
        Ok(())
    }

    /// Save the **most-restrictive** dependency kind for a package.
    /// That means if this function is called for the same package with normal and test kinds,
    /// normal kind will be saved. This is based on assumption that normal kind is more restrictive
    /// than test when filtering is applied in [`PubGrubDependencyProvider::choose_version`].
    fn save_most_restrictive_kind(&self, dep: &ManifestDependency) {
        let package = PubGrubPackage::from(dep);
        let mut write_lock = self
            .kinds
            .write()
            .expect("locking resolver state for write failed");
        if let Some(kind) = write_lock.get(&package) {
            if !dep.kind.is_test() && kind.is_test() {
                write_lock.insert(package, DepKind::Normal);
            }
        } else {
            write_lock.insert(package, dep.kind.clone());
        }
    }

    /// Check if the lockfile locks the specified registry dependency to a specific version.
    pub fn locked_registry_version<'a>(
        &'a self,
        package: &PubGrubPackage,
        range: &SemverPubgrub,
    ) -> Option<&'a PackageLock> {
        self.lockfile.packages_by_name(&package.name).find(|p| {
            p.name == package.name
                && range.contains(&p.version)
                && p.source
                    .map(|source| {
                        // Git sources are rewritten to the locked source before fetching summaries.
                        (source.is_registry()) && source.can_lock_source_id(package.source_id)
                    })
                    .unwrap_or_default()
        })
    }
}

impl DependencyProvider for PubGrubDependencyProvider {
    type P = PubGrubPackage;
    type V = Version;
    type VS = SemverPubgrub;
    type M = CustomIncompatibility;

    #[tracing::instrument(level = "trace", skip_all)]
    fn prioritize(&self, package: &Self::P, range: &Self::VS) -> Self::Priority {
        let dependency = package.to_dependency(range.clone());
        self.request_dependency(dependency);

        // Prioritize by ordering from the root.
        let priority = self
            .priority
            .read()
            .expect("locking resolver state for read failed")
            .get(package)
            .copied();
        if let Some(priority) = priority {
            return Some(PubGrubPriority::Unspecified(Reverse(priority)));
        }
        None
    }

    type Priority = Option<PubGrubPriority>;
    type Err = DependencyProviderError;

    #[tracing::instrument(level = "trace", skip(self))]
    fn choose_version(
        &self,
        package: &Self::P,
        range: &Self::VS,
    ) -> Result<Option<Self::V>, Self::Err> {
        let dependency: ManifestDependency = package.to_dependency(range.clone());
        let locked = self.locked_registry_version(package, range);
        // The lokfile does not give us any assumptions about the audit status,
        // thus we need to pull it from remote source when run in require audits mode.
        if !self.require_audits
            && let Some(locked) = locked.as_ref()
        {
            // If we are locked to some version, and it is available from cache, we do not need to
            // wait for the network query to finish, we can just return the cached summary.
            return Ok(Some(locked.version.clone()));
        }

        // Query available versions.
        let summaries = self.blocking_fetch_summaries_by_dependency(dependency)?;
        let kind = self
            .kinds
            .read()
            .expect("locking resolver state for read failed")
            .get(package)
            .cloned()
            .unwrap_or_default();
        let summaries = summaries
            .into_iter()
            .filter(|summary| range.contains(&summary.package_id.version))
            .filter(|summary| {
                !summary.yanked || self.yanked_whitelist.contains(&summary.package_id)
            })
            .map(|summary| -> Result<Option<_>, Self::Err> {
                if self.require_audits && !kind.is_test() {
                    let source_kind = &summary.package_id.source_id.kind;
                    match source_kind {
                        SourceKind::Std => {}
                        SourceKind::Registry => {
                            if !summary.audited
                                && !self
                                    .require_audits_whitelist
                                    .contains(&summary.package_id.name)
                            {
                                return Ok(None);
                            }
                        }
                        SourceKind::Path => {
                            if !self.main_package_ids.contains(&summary.package_id) {
                                return Err(
                                    DependencyProviderError::AuditRequirementInvalidSource {
                                        name: summary.package_id.name.to_string(),
                                        source_kind: source_kind.primary_field().to_string(),
                                    },
                                );
                            }
                        }
                        SourceKind::Git(_) => {
                            return Err(DependencyProviderError::AuditRequirementInvalidSource {
                                name: summary.package_id.name.to_string(),
                                source_kind: source_kind.primary_field().to_string(),
                            });
                        }
                    }
                }
                Ok(Some(summary))
            })
            .filter_map(|res| res.transpose())
            .collect::<Result<Vec<_>, Self::Err>>()?
            .into_iter()
            .sorted_by_key(|summary| summary.package_id.version.clone())
            .collect_vec();

        // Choose version.
        let summary = locked
            .and_then(|locked| {
                summaries
                    .iter()
                    .find(|summary| {
                        summary.package_id.name == locked.name
                            && summary.package_id.version == locked.version
                            && summary.package_id.source_id == locked.source.expect("source set to `None` is filtered out when searching the lockfile")
                    })
                    .cloned()
            })
            // No version locked - using the highest matching summary.
            .or_else(|| summaries.last().cloned());

        // Store retrieved summary for the selected version.
        if let Some(summary) = summary.as_ref() {
            self.packages
                .write()
                .expect("locking resolver state for write failed")
                .insert(summary.package_id, summary.clone());
        }

        Ok(summary.map(|summary| summary.package_id.version.clone()))
    }

    #[tracing::instrument(level = "trace", skip(self))]
    #[expect(clippy::type_complexity)]
    fn get_dependencies(
        &self,
        package: &Self::P,
        version: &Self::V,
    ) -> Result<Dependencies<Self::P, Self::VS, Self::M>, Self::Err> {
        let locked = self.lockfile.packages_by_name(&package.name).find(|p| {
            p.name == package.name
                && version == &p.version
                && p.source
                    .map(|source| {
                        // Git sources are rewritten to the locked source before fetching summaries.
                        (source.is_registry()) && source.can_lock_source_id(package.source_id)
                    })
                    .unwrap_or_default()
        });

        let package_id = PackageId::new(package.name.clone(), version.clone(), package.source_id);
        let self_priority = self
            .priority
            .read()
            .expect("locking resolver state for read failed")
            .get(&PubGrubPackage {
                name: package_id.name.clone(),
                source_id: package_id.source_id,
            })
            .copied();

        if let Some(locked) = locked.as_ref() {
            // If the package is locked, all of it's dependencies are locked as well.
            let dep_names = locked.dependencies.iter().collect::<HashSet<_>>();
            let deps = self
                .lockfile
                .packages()
                .filter(|package| dep_names.contains(&package.name))
                .collect_vec();

            if let Some(priority) = self_priority {
                let mut write_lock = self
                    .priority
                    .write()
                    .expect("locking resolver state for write failed");
                for dependency in deps.iter() {
                    if let Some(source_id) = dependency.source {
                        let package: PubGrubPackage = PubGrubPackage {
                            name: dependency.name.clone(),
                            source_id,
                        };
                        write_lock.insert(package, priority + 1);
                    }
                }
            }

            let deps = deps
                .iter()
                .map(|dependency| {
                    let package_id = PackageId::new(
                        dependency.name.clone(),
                        dependency.version.clone(),
                        dependency.source.expect(
                            "source set to `None` is filtered out when searching the lockfile",
                        ),
                    );
                    Ok((
                        package_id,
                        DependencyVersionReq::exact(&dependency.version.clone()),
                    ))
                })
                .collect::<Result<Vec<(PackageId, DependencyVersionReq)>, DependencyProviderError>>(
                )?;
            let constraints = deps
                .into_iter()
                .map(|(package_id, req)| (package_id.into(), req.into()))
                .collect();
            return Ok(Dependencies::Available(constraints));
        }

        // Query summary.
        let summary = self.blocking_fetch_summary_by_package_id(package_id)?;
        self.request_dependencies(&summary)?;
        // Set priority for dependencies.
        if let Some(priority) = self_priority {
            let mut write_lock = self
                .priority
                .write()
                .expect("locking resolver state for write failed");
            for dependency in summary.full_dependencies() {
                let package: PubGrubPackage = dependency.into();
                write_lock.insert(package, priority + 1);
            }
        }

        // Convert dependencies to constraints.
        let dep_filter =
            DependencyFilter::propagation(self.main_package_ids.contains(&summary.package_id));
        let deps = summary
            .filtered_full_dependencies(dep_filter)
            .map(|dependency| self.patch_map.lookup(dependency).clone())
            .map(|dependency| {
                let locked_dependency = self
                    .lockfile
                    .packages_by_name(&dependency.name)
                    .find(|p| dependency.matches_name_and_version(&p.name, &p.version))
                    .filter(|p| {
                        p.source
                            .map(|sid| {
                                (sid.is_registry()) && sid.can_lock_source_id(dependency.source_id)
                            })
                            // No locking occurs on path sources.
                            .unwrap_or(false)
                    })
                    .cloned();

                if let Some(locked_dependency) = locked_dependency.as_ref() {
                    let package_id = PackageId::new(
                        locked_dependency.name.clone(),
                        locked_dependency.version.clone(),
                        locked_dependency.source.expect(
                            "source set to `None` is filtered out when searching the lockfile",
                        ),
                    );

                    return Ok((package_id, dependency.version_req.clone()));
                }

                let original_dependency = dependency.clone();
                let dependency = rewrite_path_dependency_source_id(summary.package_id, &dependency);
                let dependency = lock_dependency(&self.lockfile, dependency)?;

                let dep_name = dependency.name.clone().to_string();
                let summaries = self.blocking_fetch_summaries_by_dependency(dependency.clone())?;
                let summaries = if summaries.is_empty() {
                    self.blocking_fetch_summaries_by_dependency(original_dependency.clone())?
                } else {
                    summaries
                };
                summaries
                    .into_iter()
                    .find(|summary| dependency.version_req.matches(&summary.package_id.version))
                    .map(|summary| (summary.package_id, dependency.version_req.clone()))
                    .ok_or_else(|| DependencyProviderError::PackageNotFound {
                        name: dep_name,
                        version: dependency.version_req.clone(),
                    })
            })
            .collect::<Result<Vec<(PackageId, DependencyVersionReq)>, DependencyProviderError>>()?;
        let constraints = deps
            .into_iter()
            .map(|(package_id, req)| (package_id.into(), req.into()))
            .collect();

        Ok(Dependencies::Available(constraints))
    }
}

impl From<DependencyVersionReq> for SemverPubgrub {
    fn from(req: DependencyVersionReq) -> Self {
        match req {
            DependencyVersionReq::Req(req) => SemverPubgrub::from(&req),
            DependencyVersionReq::Any => SemverPubgrub::empty().complement(),
            DependencyVersionReq::Locked { exact, .. } => {
                DependencyVersionReq::exact(&exact).into()
            }
        }
    }
}

/// Check lockfile for a matching package.
/// Rewrite the dependency if a matching package is found.
pub fn lock_dependency(
    lockfile: &Lockfile,
    dep: ManifestDependency,
) -> Result<ManifestDependency, DependencyProviderError> {
    if dep.source_id.is_registry() {
        // We do not rewrite to the locked version for registry dependencies
        // because they will be locked in the `choose_version` step of pubgrub.
        return Ok(dep);
    }
    lockfile
        .package_matching(dep.clone())
        .map(|locked_package_id| Ok(rewrite_locked_dependency(dep.clone(), locked_package_id?)))
        .unwrap_or(Ok(dep))
}

pub fn rewrite_locked_dependency(
    dependency: ManifestDependency,
    locked_package_id: PackageId,
) -> ManifestDependency {
    ManifestDependency::builder()
        .kind(dependency.kind.clone())
        .name(dependency.name.clone())
        .source_id(locked_package_id.source_id)
        .version_req(DependencyVersionReq::Locked {
            exact: locked_package_id.version.clone(),
            req: dependency.version_req.clone().into(),
        })
        .build()
}

pub fn rewrite_path_dependency_source_id(
    package_id: PackageId,
    dependency: &ManifestDependency,
) -> ManifestDependency {
    // Rewrite path dependencies for git sources.
    if package_id.source_id.is_git() && dependency.source_id.is_path() {
        let rewritten_dep = ManifestDependency::builder()
            .kind(dependency.kind.clone())
            .name(dependency.name.clone())
            .source_id(package_id.source_id)
            .version_req(dependency.version_req.clone())
            .build();

        return rewritten_dep;
    };
    dependency.clone()
}

/// Error thrown while trying to execute `scarb` command.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum DependencyProviderError {
    /// Package not found.
    #[error("cannot find package `{name} {version}`")]
    PackageNotFound {
        name: String,
        version: DependencyVersionReq,
    },
    /// Package query failed.
    #[error("{0}")]
    PackageQueryFailed(#[from] anyhow::Error),
    /// Channel closed.
    #[error("channel closed")]
    ChannelClosed,
    #[error(
        "dependency `{name}` from `{source_kind}` source is not allowed when audit requirement is enabled"
    )]
    AuditRequirementInvalidSource { name: String, source_kind: String },
}
