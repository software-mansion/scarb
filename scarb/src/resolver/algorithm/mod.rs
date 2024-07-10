use crate::core::lockfile::Lockfile;
use crate::core::registry::Registry;
use crate::core::{PackageId, PackageName, Resolve, Summary};
use crate::resolver::algorithm::provider::{PubGrubDependencyProvider, PubGrubPackage};
use crate::resolver::algorithm::solution::build_resolve;
use anyhow::bail;
use indoc::indoc;
use pubgrub::type_aliases::SelectedDependencies;
use std::collections::{HashMap, HashSet};
use tokio::runtime::Handle;
use tokio::task::block_in_place;

mod provider;
mod solution;

#[allow(clippy::dbg_macro)]
#[allow(dead_code)]
pub async fn resolve<'c>(
    summaries: &[Summary],
    registry: &dyn Registry,
    _lockfile: Lockfile,
    handle: &'c Handle,
) -> anyhow::Result<Resolve> {
    let main_package_ids: HashSet<PackageId> =
        HashSet::from_iter(summaries.iter().map(|sum| sum.package_id));
    block_in_place(|| {
        let summary = summaries.iter().next().unwrap();
        let package: PubGrubPackage = summary.into();
        let version = summary.package_id.version.clone();
        let provider = PubGrubDependencyProvider::new(registry, handle, main_package_ids.clone());

        let solution = pubgrub::solver::resolve(&provider, package, version)
            .map_err(|err| anyhow::format_err!("failed to resolve: {:?}", err))?;

        dbg!(&solution);

        validate_solution(&solution)?;
        build_resolve(&provider, solution)
    })
}

fn validate_solution(
    solution: &SelectedDependencies<PubGrubDependencyProvider<'_, '_>>,
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
