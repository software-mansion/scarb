use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};

use crate::core::registry::cache::RegistryCache;
use crate::core::{Config, PackageId, Summary};
use crate::resolver::pubgrub_dependency_provider::RegistryDependencyProvider;
use crate::resolver::pubgrub_types::{PubGrubPackage, PubGrubVersion};

mod pubgrub_dependency_provider;
mod pubgrub_types;

// TODO(mkaput): Produce lockfile out of this.
/// Represents a fully-resolved package dependency graph.
///
/// Each node in the graph is a package and edges represent dependencies between packages.
pub struct Resolve {
    pub package_ids: HashSet<PackageId>,
    pub targets: HashMap<PackageId, HashSet<PackageId>>,
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
    // TODO(mkaput): Parallelize this, resolve each member in parallel thread,
    //   synchronizing via RegistryCache.
    let mut package_ids = HashSet::new();
    let mut targets = HashMap::new();
    let dependency_provider = RegistryDependencyProvider::new(registry);
    for summary in summaries {
        let pubgrub_package = summary.package_id.into();
        let pubgrub_version: PubGrubVersion = summary.package_id.into();
        let solution = pubgrub::solver::resolve::<PubGrubPackage, PubGrubVersion>(
            &dependency_provider,
            pubgrub_package,
            pubgrub_version,
        )
        .map_err(|err| {
            anyhow!(
                "failed to resolve dependencies for package {}:\n{}",
                summary.package_id,
                err
            )
        })?;

        let unit_package_ids: HashSet<PackageId> = solution
            .values()
            .map(|v| v.as_package_id(&summary.package_id.name))
            .collect();
        package_ids.extend(unit_package_ids.iter());
        targets.insert(summary.package_id, unit_package_ids);
    }

    Ok(Resolve {
        package_ids,
        targets,
    })
}
