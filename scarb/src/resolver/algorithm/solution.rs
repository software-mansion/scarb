use crate::core::resolver::DependencyEdge;
use crate::core::{
    DepKind, DependencyFilter, PackageId, PackageName, Resolve, Summary, TargetKind,
};
use crate::resolver::algorithm::provider::{PubGrubDependencyProvider, PubGrubPackage};
use anyhow::bail;
use indoc::indoc;
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
            let dep_target_kind: Option<TargetKind> = match dep.kind.clone() {
                DepKind::Normal => None,
                DepKind::Target(target_kind) => Some(target_kind),
            };
            let Some(dep) = summaries.keys().find(|pid| pid.name == dep.name).copied() else {
                continue;
            };
            let weight = graph
                .edge_weight(summary.package_id, dep)
                .cloned()
                .unwrap_or_default();
            let weight = weight.extend(dep_target_kind);
            graph.add_edge(summary.package_id, dep, weight);
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
            bail!(
                indoc! {"
                    found dependencies on the same package `{}` coming from incompatible \
                    sources:
                    source 1: {}
                    source 2: {}
                "},
                pkg.name,
                existing.source_id,
                pkg.source_id
            );
        }
        seen.insert(pkg.name.clone(), pkg.clone());
    }
    Ok(())
}
