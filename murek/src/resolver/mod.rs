use std::collections::HashMap;

use anyhow::{anyhow, Result};

use crate::core::package::PackageName;
use crate::core::registry::Registry;
use crate::core::{Config, Package, PackageId, Summary};
use crate::internal::asyncx::AwaitSync;

// TODO(mkaput): Produce lockfile out of this.
/// Represents a fully-resolved package dependency graph.
///
/// Each node in the graph is a package and edges represent dependencies between packages.
pub struct Resolve {
    pub packages: HashMap<PackageName, PackageId>,
    pub summaries: HashMap<PackageId, Summary>,
}

impl Resolve {
    pub fn package_ids(&self) -> impl Iterator<Item = PackageId> + '_ {
        self.packages.values().copied()
    }

    /// Gather [`Package`] instances from this resolver result, by asking the [`Registry`]
    /// to download resolved packages.
    ///
    /// Currently, it is expected that all packages are already downloaded during resolution,
    /// so the `download` calls in this method should be cheap, but this may change the future.
    pub fn download_packages(
        &self,
        registry: &mut Registry<'_>,
    ) -> Result<HashMap<PackageId, Package>> {
        let resolved_package_ids = self.package_ids().collect::<Vec<_>>();
        let packages = registry.download_many(&resolved_package_ids).await_sync()?;
        let packages = HashMap::from_iter(packages.into_iter().map(|pkg| (pkg.id, pkg)));
        Ok(packages)
    }
}

/// Builds the list of all packages required to build the first argument.
///
/// # Arguments
///
/// * `summaries` - the list of all top-level packages that are intended to be part of
///     the lock file (resolve output).
///     These typically are a list of all workspace members.
///
/// * `registry` - this is the source from which all package summaries are loaded.
///     It is expected that this is extensively configured ahead of time and is idempotent with
///     our requests to it (aka returns the same results for the same query every time).
///
/// * `config` - [`Config`] object.
pub fn resolve(
    summaries: &[Summary],
    registry: &mut Registry<'_>,
    _config: &Config,
) -> Result<Resolve> {
    let mut packages = HashMap::from_iter(
        summaries
            .iter()
            .map(|s| (s.package_id.name.clone(), s.package_id)),
    );
    let mut summaries = HashMap::from_iter(summaries.iter().map(|s| (s.package_id, s.clone())));

    // TODO(mkaput): This is very bad, use PubGrub here.
    let mut queue: Vec<PackageId> = summaries.keys().copied().collect();
    while !queue.is_empty() {
        let mut next_queue = Vec::new();

        for package_id in queue {
            for dep in summaries[&package_id].dependencies.clone() {
                if packages.contains_key(&dep.name) {
                    continue;
                }

                let results = registry.query(&dep).await_sync()?;

                let dep_summary = results
                    .first()
                    .ok_or_else(|| anyhow!("cannot find package {}", dep.name))?;

                packages.insert(dep_summary.package_id.name.clone(), dep_summary.package_id);
                summaries.insert(dep_summary.package_id, dep_summary.clone());
                next_queue.push(dep_summary.package_id);
            }
        }

        queue = next_queue;
    }

    Ok(Resolve {
        packages,
        summaries,
    })
}
