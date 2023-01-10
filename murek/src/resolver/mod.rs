use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};

use crate::core::package::PackageName;
use crate::core::registry::cache::RegistryCache;
use crate::core::{Config, PackageId, Summary};
use crate::internal::asyncx::AwaitSync;

mod compilation_units;

// TODO(mkaput): Produce lockfile out of this.
/// Represents a fully-resolved package dependency graph.
///
/// Each node in the graph is a package and edges represent dependencies between packages.
pub struct Resolve {
    pub package_ids: HashSet<PackageId>,
    pub compilation_units: HashMap<PackageId, HashSet<PackageId>>,
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
    registry: &mut RegistryCache<'_>,
    _config: &Config,
) -> Result<Resolve> {
    let mut packages = HashMap::from_iter(
        summaries
            .iter()
            .map(|s| (s.package_id.name.clone(), s.package_id)),
    );
    let mut summaries: HashMap<_, _> = summaries
        .iter()
        .map(|s| (s.package_id, s.clone()))
        .collect();

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

    let package_ids = packages.values().copied().collect();

    let compilation_units = compilation_units::collect(
        summaries
            .values()
            .map(|summary| CUNode::new(summary, &packages)),
    );

    Ok(Resolve {
        package_ids,
        compilation_units,
    })
}

struct CUNode {
    package_id: PackageId,
    dependencies: HashSet<PackageId>,
}

impl CUNode {
    fn new(summary: &Summary, packages: &HashMap<PackageName, PackageId>) -> Self {
        Self {
            package_id: summary.package_id,
            dependencies: summary
                .dependencies
                .iter()
                .map(|dep| packages[&dep.name])
                .collect(),
        }
    }
}

impl compilation_units::Node for CUNode {
    type Id = PackageId;

    fn id(&self) -> Self::Id {
        self.package_id
    }

    fn direct_dependencies(&self) -> &HashSet<Self::Id> {
        &self.dependencies
    }
}
