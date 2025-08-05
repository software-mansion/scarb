use crate::core::PackageName;
use crate::resolver::provider::{PubGrubDependencyProvider, PubGrubPackage};
use anyhow::bail;
use indoc::indoc;
use itertools::Itertools;
use pubgrub::SelectedDependencies;
use std::collections::HashMap;

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
