use crate::core::lockfile::Lockfile;
use crate::core::registry::Registry;
use crate::core::{PackageId, PackageName, Resolve, Summary};
use crate::resolver::algorithm::provider::{PubGrubDependencyProvider, PubGrubPackage};
use crate::resolver::algorithm::solution::build_resolve;
use anyhow::bail;
use indoc::indoc;
use itertools::Itertools;
use pubgrub::type_aliases::SelectedDependencies;
use pubgrub::{Incompatibility, State};
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
        let provider = PubGrubDependencyProvider::new(registry, handle, main_package_ids);

        // Init state
        let main_package_ids = provider
            .main_package_ids()
            .clone()
            .into_iter()
            .collect_vec();
        let Some((first, rest)) = main_package_ids.split_first() else {
            bail!("empty summaries");
        };
        let package: PubGrubPackage = (*first).into();
        let version = first.version.clone();
        let mut state = State::init(package.clone(), version);
        state
            .unit_propagation(package.clone())
            .map_err(|err| anyhow::format_err!("unit propagation failed: {:?}", err))?;
        for package_id in rest {
            let package: PubGrubPackage = (*package_id).into();
            let version = package_id.version.clone();
            state.add_incompatibility(Incompatibility::not_root(package.clone(), version.clone()));
            state
                .unit_propagation(package)
                .map_err(|err| anyhow::format_err!("unit propagation failed: {:?}", err))?
        }

        // Resolve requirements
        let solution = pubgrub::solver::resolve_state(&provider, &mut state, package)
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
