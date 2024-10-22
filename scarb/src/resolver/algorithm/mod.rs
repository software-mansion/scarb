use crate::core::lockfile::Lockfile;
use crate::core::registry::Registry;
use crate::core::{PackageId, Resolve, Summary};
use crate::resolver::algorithm::provider::{
    rewrite_dependency_source_id, rewrite_locked_dependency, DependencyProviderError,
    PubGrubDependencyProvider, PubGrubPackage,
};
use crate::resolver::algorithm::solution::{build_resolve, validate_solution};
use crate::resolver::algorithm::state::{Request, ResolverState};
use anyhow::bail;
use futures::{FutureExt, TryFutureExt};
use itertools::Itertools;
use pubgrub::error::PubGrubError;
use pubgrub::report::{DefaultStringReporter, Reporter};
use pubgrub::{Incompatibility, State};
use std::collections::HashSet;
use std::sync::Arc;
use std::thread;
use tokio::sync::{mpsc, oneshot};

mod in_memory_index;
mod provider;
mod solution;
mod state;

#[allow(clippy::dbg_macro)]
#[allow(dead_code)]
pub async fn resolve<'c>(
    summaries: &[Summary],
    registry: &dyn Registry,
    lockfile: Lockfile,
) -> anyhow::Result<Resolve> {
    let state = Arc::new(ResolverState::default());

    let (request_sink, request_stream): (mpsc::Sender<Request>, mpsc::Receiver<Request>) =
        mpsc::channel(300);

    let requests_fut = state
        .clone()
        .fetch(registry, request_stream)
        .map_err(|err| anyhow::format_err!(err))
        .fuse();

    for summary in summaries {
        let package: PubGrubPackage = summary.package_id.into();
        if state.index.packages().register(package.clone()) {
            request_sink.send(Request::Package(package)).await?;
        }
        for dep in summary.full_dependencies() {
            let dep = rewrite_dependency_source_id(summary.package_id, dep)?;
            let locked_package_id = lockfile.packages_matching(dep.clone());
            let dep = if let Some(locked_package_id) = locked_package_id {
                rewrite_locked_dependency(dep.clone(), locked_package_id?)
            } else {
                dep.clone()
            };

            let package: PubGrubPackage = (&dep).into();
            if state.index.packages().register(package.clone()) {
                request_sink.send(Request::Package(package)).await?;
            }
        }
    }

    let main_package_ids: HashSet<PackageId> =
        HashSet::from_iter(summaries.iter().map(|sum| sum.package_id));

    let (tx, rx) = oneshot::channel();

    let cloned_lockfile = lockfile.clone();
    thread::Builder::new()
        .name("scarb-resolver".into())
        .spawn(move || {
            let result = || {
                let provider = PubGrubDependencyProvider::new(
                    main_package_ids,
                    state,
                    request_sink,
                    cloned_lockfile,
                );

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
                    state.add_incompatibility(Incompatibility::not_root(
                        package.clone(),
                        version.clone(),
                    ));
                    state
                        .unit_propagation(package)
                        .map_err(|err| anyhow::format_err!("unit propagation failed: {:?}", err))?
                }

                // Resolve requirements
                let solution = pubgrub::solver::resolve_state(&provider, &mut state, package)
                    .map_err(format_error)?;

                validate_solution(&solution)?;
                build_resolve(&provider, solution)
            };
            let result = result();
            tx.send(result).unwrap();
        })?;

    let resolve_fut = async move {
        rx.await
            .map_err(|_| DependencyProviderError::ChannelClosed.into())
            .and_then(|result| result)
    };

    let (_, resolve) = tokio::try_join!(requests_fut, resolve_fut)?;
    resolve.check_checksums(&lockfile)?;
    Ok(resolve)
}

fn format_error(err: PubGrubError<PubGrubDependencyProvider>) -> anyhow::Error {
    match err {
        PubGrubError::NoSolution(derivation_tree) => {
            anyhow::format_err!(
                "version solving failed:\n{}\n",
                DefaultStringReporter::report(&derivation_tree)
            )
        }
        PubGrubError::ErrorChoosingPackageVersion(DependencyProviderError::PackageNotFound {
            name,
            version,
        }) => {
            anyhow::format_err!("cannot find package `{name} {version}`")
        }
        PubGrubError::ErrorChoosingPackageVersion(DependencyProviderError::PackageQueryFailed(
            err,
        )) => anyhow::format_err!("{}", err).context("dependency query failed"),
        PubGrubError::ErrorRetrievingDependencies {
            package,
            version,
            source,
        } => anyhow::Error::from(source)
            .context(format!("cannot get dependencies of `{package}@{version}`")),
        PubGrubError::SelfDependency { package, version } => {
            anyhow::format_err!("self dependency found: `{}@{}`", package, version)
        }
        PubGrubError::ErrorInShouldCancel(err) => {
            anyhow::format_err!("{}", err).context("should cancel failed")
        }
        PubGrubError::Failure(msg) => anyhow::format_err!("{}", msg).context("resolver failure"),
        PubGrubError::ErrorChoosingPackageVersion(DependencyProviderError::ChannelClosed) => {
            anyhow::format_err!("channel closed")
        }
    }
}
