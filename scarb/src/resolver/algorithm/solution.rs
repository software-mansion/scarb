use crate::core::resolver::DependencyEdge;
use crate::core::{DepKind, DependencyFilter, PackageId, PackageName, Resolve, Summary};
use crate::resolver::algorithm::provider::{PubGrubDependencyProvider, PubGrubPackage};
use anyhow::bail;
use indoc::indoc;
use itertools::Itertools;
use petgraph::prelude::DiGraphMap;
use pubgrub::SelectedDependencies;
use std::collections::HashMap;

/// This translates the representation of the solution produced by the PubGrub algorithm into the
/// representation used by Scarb internally.
pub fn build_resolve(
    provider: &PubGrubDependencyProvider,
    solution: SelectedDependencies<PubGrubDependencyProvider>,
) -> anyhow::Result<Resolve> {
    let summaries: HashMap<PackageId, Summary> = solution
        .into_iter()
        .map(|(package, version)| {
            let pid = PackageId::new(package.name.clone(), version.clone(), package.source_id);
            let sum = provider
                .fetch_summary(pid)
                .map_err(|err| anyhow::format_err!("failed to get summary: {:?}", err))?;
            Ok((sum.package_id, sum))
        })
        .collect::<anyhow::Result<HashMap<_, _>>>()?;

    let mut graph: DiGraphMap<PackageId, DependencyEdge> = Default::default();

    for pid in summaries.keys() {
        graph.add_node(*pid);
    }

    for summary in summaries.values() {
        let dep_filter = DependencyFilter::propagation(
            provider.main_package_ids().contains(&summary.package_id),
        );
        for dep in summary.filtered_full_dependencies(dep_filter) {
            let dep_kind = dep.kind.clone();
            let Some(dep) = summaries.keys().find(|pid| pid.name == dep.name).copied() else {
                continue;
            };
            let weight = graph.edge_weight_mut(summary.package_id, dep);
            if let Some(weight) = weight {
                match dep_kind {
                    DepKind::Normal => {
                        weight.accept_all();
                    }
                    DepKind::Target(target_kind) => {
                        weight.accept_new(target_kind);
                    }
                };
            } else {
                let weight = match dep_kind {
                    DepKind::Normal => DependencyEdge::for_all_targets(),
                    DepKind::Target(target_kind) => DependencyEdge::for_target(target_kind),
                };
                graph.add_edge(summary.package_id, dep, weight);
            }
        }
    }

    Ok(Resolve { graph, summaries })
}

/// This function validates the solution produced by the PubGrub algorithm according to custom
/// rules of the Scarb dependency model (that are not represented by the version requirements themselves).
pub fn validate_solution(
    solution: &SelectedDependencies<PubGrubDependencyProvider>,
) -> anyhow::Result<()> {
    // Same package, different sources.
    let mut seen: HashMap<PackageName, PubGrubPackage> = Default::default();
    for pkg in solution.keys() {
        if let Some(existing) = seen.get(&pkg.name) {
            let source_ids = vec![existing.source_id, pkg.source_id]
                .into_iter()
                .sorted()
                .collect_vec();
            bail!(
                indoc! {"
                    found dependencies on the same package `{}` coming from incompatible \
                    sources:
                    source 1: {}
                    source 2: {}
                "},
                pkg.name,
                source_ids[0],
                source_ids[1]
            );
        }
        seen.insert(pkg.name.clone(), pkg.clone());
    }
    Ok(())
}
