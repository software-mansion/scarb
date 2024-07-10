use crate::core::registry::Registry;
use crate::core::{
    DependencyFilter, DependencyVersionReq, ManifestDependency, PackageId, PackageName, SourceId,
    Summary,
};
use itertools::Itertools;
use pubgrub::solver::{Dependencies, DependencyProvider};
use pubgrub::version_set::VersionSet;
use semver::{Version, VersionReq};
use semver_pubgrub::SemverPubgrub;
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::sync::RwLock;
use thiserror::Error;
use tokio::runtime::Handle;

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct CustomIncompatibility(String);

impl Display for CustomIncompatibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PubGrubPackage {
    pub name: PackageName,
    pub source_id: SourceId,
}

impl From<&PubGrubPackage> for ManifestDependency {
    fn from(package: &PubGrubPackage) -> Self {
        ManifestDependency::builder()
            .name(package.name.clone())
            .source_id(package.source_id)
            .version_req(DependencyVersionReq::Any)
            .build()
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

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PubGrubPriority {
    /// The package has no specific priority.
    ///
    /// As such, its priority is based on the order in which the packages were added (FIFO), such
    /// that the first package we visit is prioritized over subsequent packages.
    ///
    /// TODO(charlie): Prefer constrained over unconstrained packages, if they're at the same depth
    /// in the dependency graph.
    Unspecified(Reverse<usize>),

    /// The version range is constrained to a single version (e.g., with the `==` operator).
    Singleton(Reverse<usize>),

    /// The package was specified via a direct URL.
    DirectUrl(Reverse<usize>),

    /// The package is the root package.
    Root,
}

pub struct PubGrubDependencyProvider<'a, 'c> {
    handle: &'c Handle,
    registry: &'a dyn Registry,
    priority: RwLock<HashMap<PubGrubPackage, usize>>,
    packages: RwLock<HashMap<PackageId, Summary>>,
    main_package_ids: HashSet<PackageId>,
}

impl<'a, 'c> PubGrubDependencyProvider<'a, 'c> {
    pub fn new(
        registry: &'a dyn Registry,
        handle: &'c Handle,
        main_package_ids: HashSet<PackageId>,
    ) -> Self {
        Self {
            handle,
            registry,
            main_package_ids,
            priority: RwLock::new(HashMap::new()),
            packages: RwLock::new(HashMap::new()),
        }
    }

    pub fn main_package_ids(&self) -> &HashSet<PackageId> {
        &self.main_package_ids
    }

    pub fn fetch_summary(&self, package_id: PackageId) -> Result<Summary, DependencyProviderError> {
        let summary = self.packages.read().unwrap().get(&package_id).cloned();
        summary.map(Ok).unwrap_or_else(|| {
            let dependency = ManifestDependency::builder()
                .name(package_id.name.clone())
                .source_id(package_id.source_id)
                .version_req(DependencyVersionReq::exact(&package_id.version))
                .build();
            let summary = self
                .handle
                .block_on(self.registry.query(&dependency))
                .map_err(DependencyProviderError::PackageQueryFailed)?
                .into_iter()
                .find_or_first(|summary| summary.package_id == package_id);
            if let Some(summary) = summary.as_ref() {
                let mut write_lock = self.packages.write().unwrap();
                write_lock.insert(summary.package_id, summary.clone());
                write_lock.insert(package_id, summary.clone());
            }
            summary.ok_or_else(|| {
                DependencyProviderError::PackageNotFound(dependency.name.clone().to_string())
            })
        })
    }

    fn query(
        &self,
        dependency: ManifestDependency,
    ) -> Result<Vec<Summary>, DependencyProviderError> {
        let summaries = self
            .handle
            .block_on(self.registry.query(&dependency))
            .map_err(DependencyProviderError::PackageQueryFailed)?;

        {
            let mut write_lock = self.packages.write().unwrap();
            for summary in summaries.iter() {
                write_lock.insert(summary.package_id, summary.clone());
            }
        }

        // Sort from highest to lowest.
        let summaries = summaries
            .into_iter()
            .sorted_by_key(|sum| sum.package_id.version.clone())
            .rev()
            .collect_vec();

        Ok(summaries)
    }
}

impl<'a, 'c> DependencyProvider for PubGrubDependencyProvider<'a, 'c> {
    type P = PubGrubPackage;
    type V = Version;
    type VS = SemverPubgrub;
    type M = CustomIncompatibility;

    fn prioritize(&self, package: &Self::P, _range: &Self::VS) -> Self::Priority {
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
        let dependency: ManifestDependency = package.into();
        let summaries = self.query(dependency)?;

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
        let summary = self.fetch_summary(package_id)?;

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
                let req = VersionReq::from(dependency.version_req.clone());
                let dep_name = dependency.name.clone().to_string();
                let summaries = self.query(dependency)?;
                summaries
                    .into_iter()
                    .find(|summary| req.matches(&summary.package_id.version))
                    .map(|summary| (summary, req))
                    .ok_or_else(|| DependencyProviderError::PackageNotFound(dep_name))
            })
            .collect::<Result<Vec<(Summary, VersionReq)>, DependencyProviderError>>()?;
        let constraints = deps
            .into_iter()
            .map(|(summary, req)| (summary.package_id.into(), SemverPubgrub::from(&req)))
            .collect();

        Ok(Dependencies::Available(constraints))
    }
}

/// Error thrown while trying to execute `scarb` command.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum DependencyProviderError {
    /// Package not found.
    #[error("failed to get package `{0}`")]
    PackageNotFound(String),
    // Package query failed.
    #[error("package query failed: {0}")]
    PackageQueryFailed(#[from] anyhow::Error),
}
