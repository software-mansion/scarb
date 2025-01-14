use crate::core::lockfile::Lockfile;
use crate::core::{
    DependencyFilter, DependencyVersionReq, ManifestDependency, PackageId, PackageName, SourceId,
    Summary,
};
use crate::resolver::algorithm::in_memory_index::VersionsResponse;
use crate::resolver::algorithm::{Request, ResolverState};
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
    main_package_ids: HashSet<PackageId>,
    lockfile: Lockfile,
    state: Arc<ResolverState>,
    request_sink: mpsc::Sender<Request>,
}

impl PubGrubDependencyProvider {
    pub fn new(
        main_package_ids: HashSet<PackageId>,
        state: Arc<ResolverState>,
        request_sink: mpsc::Sender<Request>,
        lockfile: Lockfile,
    ) -> Self {
        Self {
            main_package_ids,
            priority: RwLock::new(HashMap::new()),
            packages: RwLock::new(HashMap::new()),
            state,
            lockfile,
            request_sink,
        }
    }

    pub fn main_package_ids(&self) -> &HashSet<PackageId> {
        &self.main_package_ids
    }

    pub fn fetch_summary_and_request_dependencies(
        &self,
        package_id: PackageId,
    ) -> Result<Summary, DependencyProviderError> {
        let summary = self.fetch_summary(package_id)?;
        self.request_dependencies(&summary)?;
        Ok(summary)
    }

    pub fn fetch_summary(&self, package_id: PackageId) -> Result<Summary, DependencyProviderError> {
        let summary = self.packages.read().unwrap().get(&package_id).cloned();
        let summary = summary.map(Ok).unwrap_or_else(|| {
            let dependency = ManifestDependency::builder()
                .name(package_id.name.clone())
                .source_id(package_id.source_id)
                .version_req(DependencyVersionReq::exact(&package_id.version))
                .build();
            let summary = self
                .wait_for_summaries(dependency.clone())?
                .into_iter()
                .find_or_first(|summary| summary.package_id == package_id);
            if let Some(summary) = summary.as_ref() {
                let mut write_lock = self.packages.write().unwrap();
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

    fn request_dependencies(&self, summary: &Summary) -> Result<(), DependencyProviderError> {
        for dep in summary.dependencies.iter() {
            let locked_package_id = self.lockfile.packages_matching(dep.clone());
            let dep = if let Some(locked_package_id) = locked_package_id {
                rewrite_locked_dependency(dep.clone(), locked_package_id?)
            } else {
                dep.clone()
            };

            if self.state.index.packages().register(dep.clone()) {
                self.request_sink
                    .blocking_send(Request::Package(dep.clone()))
                    .unwrap();
            }

            let dep = rewrite_path_dependency_source_id(summary.package_id, &dep)?;
            if self.state.index.packages().register(dep.clone()) {
                self.request_sink
                    .blocking_send(Request::Package(dep))
                    .unwrap();
            }
        }
        Ok(())
    }

    fn wait_for_summaries(
        &self,
        dependency: ManifestDependency,
    ) -> Result<Vec<Summary>, DependencyProviderError> {
        let summaries = self
            .state
            .index
            .packages()
            .wait_blocking(&dependency)
            .unwrap();
        let VersionsResponse::Found(summaries) = summaries.as_ref();

        {
            let mut write_lock = self.packages.write().unwrap();
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
}

impl DependencyProvider for PubGrubDependencyProvider {
    type P = PubGrubPackage;
    type V = Version;
    type VS = SemverPubgrub;
    type M = CustomIncompatibility;

    fn prioritize(&self, package: &Self::P, range: &Self::VS) -> Self::Priority {
        let dependency: ManifestDependency = package.to_dependency(range.clone());
        if self.state.index.packages().register(dependency.clone()) {
            self.request_sink
                .blocking_send(Request::Package(dependency.clone()))
                .unwrap();
        }

        // Prioritize by ordering from root.
        let priority = self.priority.read().unwrap().get(package).copied();
        if let Some(priority) = priority {
            return Some(PubGrubPriority::Unspecified(Reverse(priority)));
        }
        None
    }

    type Priority = Option<PubGrubPriority>;
    type Err = DependencyProviderError;

    fn choose_version(
        &self,
        package: &Self::P,
        range: &Self::VS,
    ) -> Result<Option<Self::V>, Self::Err> {
        // Query available versions.
        let dependency: ManifestDependency = package.to_dependency(range.clone());
        let summaries = self.wait_for_summaries(dependency)?;

        // Choose version.
        let summary = summaries
            .into_iter()
            .find(|summary| range.contains(&summary.package_id.version));

        // Store retrieved summary for selected version.
        if let Some(summary) = summary.as_ref() {
            self.packages
                .write()
                .unwrap()
                .insert(summary.package_id, summary.clone());
        }

        Ok(summary.map(|summary| summary.package_id.version.clone()))
    }

    fn get_dependencies(
        &self,
        package: &Self::P,
        version: &Self::V,
    ) -> Result<Dependencies<Self::P, Self::VS, Self::M>, Self::Err> {
        // Query summary.
        let package_id = PackageId::new(package.name.clone(), version.clone(), package.source_id);
        let summary = self.fetch_summary_and_request_dependencies(package_id)?;

        // Set priority for dependencies.
        let self_priority = self
            .priority
            .read()
            .unwrap()
            .get(&PubGrubPackage {
                name: package_id.name.clone(),
                source_id: package_id.source_id,
            })
            .copied();
        if let Some(priority) = self_priority {
            let mut write_lock = self.priority.write().unwrap();
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
            .cloned()
            .map(|dependency| {
                let original_dep = dependency.clone();
                let dependency =
                    rewrite_path_dependency_source_id(summary.package_id, &dependency)?;
                let locked_package_id = self.lockfile.packages_matching(dependency.clone());
                let dependency = if let Some(locked_package_id) = locked_package_id {
                    rewrite_locked_dependency(dependency.clone(), locked_package_id?)
                } else {
                    dependency
                };

                let dep_name = dependency.name.clone().to_string();
                let summaries = self.wait_for_summaries(dependency.clone())?;
                let summaries = if summaries.is_empty() {
                    self.wait_for_summaries(original_dep.clone())?
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
) -> Result<ManifestDependency, DependencyProviderError> {
    // Rewrite path dependencies for git sources.
    if package_id.source_id.is_git() && dependency.source_id.is_path() {
        let rewritten_dep = ManifestDependency::builder()
            .kind(dependency.kind.clone())
            .name(dependency.name.clone())
            .source_id(package_id.source_id)
            .version_req(dependency.version_req.clone())
            .build();

        return Ok(rewritten_dep);
    };
    Ok(dependency.clone())
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
}
