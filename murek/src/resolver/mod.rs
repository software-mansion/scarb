use std::collections::HashMap;

use anyhow::{anyhow, Result};
use petgraph::graphmap::DiGraphMap;

use crate::core::registry::cache::RegistryCache;
use crate::core::resolver::Resolve;
use crate::core::{Config, PackageId, Summary};

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
#[tracing::instrument(level = "trace", skip_all)]
pub async fn resolve(
    summaries: &[Summary],
    registry: &mut RegistryCache<'_>,
    _config: &Config,
) -> Result<Resolve> {
    let mut graph = DiGraphMap::new();

    let mut packages: HashMap<_, _> = HashMap::from_iter(
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
            graph.add_node(package_id);

            for dep in summaries[&package_id].dependencies.clone() {
                if packages.contains_key(&dep.name) {
                    continue;
                }

                let results = registry.query(&dep).await?;

                let dep_summary = results
                    .first()
                    .ok_or_else(|| anyhow!("cannot find package {}", dep.name))?;

                let dep_package_id = dep_summary.package_id;

                graph.add_edge(package_id, dep_package_id, ());
                packages.insert(dep_package_id.name.clone(), dep_package_id);
                summaries.insert(dep_package_id, dep_summary.clone());
                next_queue.push(dep_package_id);
            }
        }

        queue = next_queue;
    }

    Ok(Resolve { graph })
}
