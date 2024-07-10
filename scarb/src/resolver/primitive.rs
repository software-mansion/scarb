#![allow(dead_code)]

use crate::core::lockfile::Lockfile;
use crate::core::registry::Registry;
use crate::core::resolver::DependencyEdge;
use crate::core::{
    DepKind, DependencyFilter, DependencyVersionReq, ManifestDependency, PackageId, Resolve,
    Summary, TargetKind,
};
use anyhow::bail;
use indoc::{formatdoc, indoc};
use petgraph::graphmap::DiGraphMap;
use std::collections::{HashMap, HashSet};
use tokio::runtime::Handle;

#[tracing::instrument(level = "trace", skip_all)]
pub async fn resolve<'c>(
    summaries: &[Summary],
    registry: &dyn Registry,
    lockfile: Lockfile,
    _handle: &'c Handle,
) -> anyhow::Result<Resolve> {
    // TODO(#2): This is very bad, use PubGrub here.
    let mut graph = DiGraphMap::<PackageId, DependencyEdge>::new();

    let main_packages = summaries
        .iter()
        .map(|sum| sum.package_id)
        .collect::<HashSet<PackageId>>();
    let mut packages: HashMap<_, _> = HashMap::from_iter(
        summaries
            .iter()
            .map(|s| (s.package_id.name.clone(), s.package_id)),
    );

    let mut summaries: HashMap<_, _> = summaries
        .iter()
        .map(|s| (s.package_id, s.clone()))
        .collect();

    let mut queue: Vec<PackageId> = summaries.keys().copied().collect();
    while !queue.is_empty() {
        let mut next_queue = Vec::new();

        for package_id in queue {
            graph.add_node(package_id);

            let summary = summaries[&package_id].clone();
            let dep_filter =
                DependencyFilter::propagation(main_packages.contains(&summary.package_id));
            for dep in summary.filtered_full_dependencies(dep_filter) {
                let dep = rewrite_dependency_source_id(registry, &package_id, dep).await?;

                let locked_package_id = lockfile.packages_matching(dep.clone());
                let dep = if let Some(locked_package_id) = locked_package_id {
                    rewrite_locked_dependency(dep.clone(), locked_package_id?)
                } else {
                    dep
                };

                let results = registry.query(&dep).await?;

                let Some(dep_summary) = results.first() else {
                    bail!("cannot find package {}", dep.name)
                };

                let dep_target_kind: Option<TargetKind> = match dep.kind.clone() {
                    DepKind::Normal => None,
                    DepKind::Target(target_kind) => Some(target_kind),
                };
                let dep = dep_summary.package_id;

                if let Some(existing) = packages.get(dep.name.as_ref()) {
                    if existing.source_id != dep.source_id {
                        bail!(
                            indoc! {"
                            found dependencies on the same package `{}` coming from incompatible \
                            sources:
                            source 1: {}
                            source 2: {}
                            "},
                            dep.name,
                            existing.source_id,
                            dep.source_id
                        );
                    }
                }

                let weight = graph
                    .edge_weight(package_id, dep)
                    .cloned()
                    .unwrap_or_default();
                let weight = weight.extend(dep_target_kind);
                graph.add_edge(package_id, dep, weight);
                summaries.insert(dep, dep_summary.clone());

                if packages.contains_key(dep.name.as_ref()) {
                    continue;
                }

                packages.insert(dep.name.clone(), dep);
                next_queue.push(dep);
            }
        }

        queue = next_queue;
    }

    // Detect incompatibilities and bail in case ones are found.
    let mut incompatibilities = Vec::new();
    for from_package in graph.nodes() {
        let dep_filter = DependencyFilter::propagation(main_packages.contains(&from_package));
        for manifest_dependency in summaries[&from_package].filtered_full_dependencies(dep_filter) {
            let to_package = packages[&manifest_dependency.name];
            if !manifest_dependency.matches_package_id(to_package) {
                let message = format!(
                    "- {from_package} cannot use {to_package}, because {} requires {} {}",
                    from_package.name, to_package.name, manifest_dependency.version_req
                );
                incompatibilities.push(message);
            }
        }
    }

    if !incompatibilities.is_empty() {
        incompatibilities.sort();
        let incompatibilities = incompatibilities.join("\n");
        bail!(formatdoc! {"
            Version solving failed:
            {incompatibilities}

            Scarb does not have real version solving algorithm yet.
            Perhaps in the future this conflict could be resolved, but currently,
            please upgrade your dependencies to use latest versions of their dependencies.
        "});
    }

    let resolve = Resolve { graph, summaries };
    resolve.check_checksums(&lockfile)?;
    Ok(resolve)
}

fn rewrite_locked_dependency(
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

async fn rewrite_dependency_source_id(
    registry: &dyn Registry,
    package_id: &PackageId,
    dependency: &ManifestDependency,
) -> anyhow::Result<ManifestDependency> {
    // Rewrite path dependencies for git sources.
    if package_id.source_id.is_git() && dependency.source_id.is_path() {
        let rewritten_dep = ManifestDependency::builder()
            .kind(dependency.kind.clone())
            .name(dependency.name.clone())
            .source_id(package_id.source_id)
            .version_req(dependency.version_req.clone())
            .build();
        // Check if this dependency can be queried from git source.
        // E.g. packages below other package's manifest will not be accessible.
        if !registry.query(&rewritten_dep).await?.is_empty() {
            // If it is, return rewritten dependency.
            return Ok(rewritten_dep);
        }
    };

    Ok(dependency.clone())
}
