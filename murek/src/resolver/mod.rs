//! Implementation of the [PubGrub] version solving algorithm for Scarb.
//!
//! This implementation is heavily based on two other ones, to such degree that some portions of
//! their source code has been copied here unchanged:
//!
//! 1. [`hex_solver`] from Elixir's Hex package manager,
//! 2. [`pubgrub`] Rust crate.
//!
//! [PubGrub]: https://nex3.medium.com/pubgrub-2fb6470504f
//! [`hex_solver`]: https://github.com/hexpm/hex_solver
//! [`pubgrub`]: https://github.com/pubgrub-rs/pubgrub

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use petgraph::graphmap::DiGraphMap;

use crate::core::registry::Registry;
use crate::core::resolver::Resolve;
use crate::core::{Config, PackageId, Summary};

mod incompatibility;
mod incompatibility_set;
mod package_range;
mod package_ref;
mod term;
mod version_constraint;

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
///     It is also advised to implement internal caching, as the resolver may frequently ask
///     repetitive queries.
///
/// * `config` - [`Config`] object.
#[tracing::instrument(level = "trace", skip_all)]
pub async fn resolve(
    summaries: &[Summary],
    registry: &mut dyn Registry,
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

#[cfg(test)]
mod tests {
    use crate::core::registry::mock::MockRegistry;

    fn run(registry: &str, query: impl IntoIterator<Item = (&str, &str)>) -> String {
        let registry = MockRegistry::from_toml(registry);
    }

    #[test]
    fn nested_deps() {
        let registry = r#"
            [foo]
            version = "1.0.0"

            [foo.dependencies]
            bar = "=1.0.0"

            [bar]
            version = "1.0.0"
        "#;

        assert_eq!(
            &run(registry, [("bar", "1.0.0")]),
            r#"
                bar v1.0.0
                foo v1.0.0
            "#
        );
    }
}
