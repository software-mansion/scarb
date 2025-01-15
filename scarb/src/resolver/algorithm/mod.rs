use crate::core::lockfile::Lockfile;
use crate::core::registry::Registry;
use crate::core::{PackageId, Resolve, Summary};
use crate::resolver::algorithm::provider::{
    rewrite_locked_dependency, DependencyProviderError, PubGrubDependencyProvider, PubGrubPackage,
};
use crate::resolver::algorithm::solution::{build_resolve, validate_solution};
use crate::resolver::algorithm::state::{Request, ResolverState};
use anyhow::{bail, format_err, Error};
use futures::{FutureExt, TryFutureExt};
use itertools::Itertools;
use pubgrub::PubGrubError;
use pubgrub::{DefaultStringReporter, Reporter};
use pubgrub::{Incompatibility, State};
use std::collections::HashSet;
use std::sync::Arc;
use std::thread;
use tokio::sync::{mpsc, oneshot};

mod in_memory_index;
mod provider;
mod solution;
mod state;

/// Builds the list of all packages required to build the first argument.
///
/// This is meant to be used by `scarb::resolver::resolve`.
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
/// * `lockfile` - a [`Lockfile`] instance, which is used to guide the resolution process. Empty
///     lockfile will result in no guidance. This function does not read or write lock files from
///     the filesystem.
///
/// # Implementation:
///
/// This solution uses the PubGrub version solving algorithm to resolve the dependencies.
/// It is supposed to be very fast and to explain errors more clearly than the alternatives.
///
/// The implementation is based on the Rust crate implementing the PubGrub, from `pubgrub-rs/pubgrub`.
///
/// The implementation consists of two main components:
/// - `ResolverState` responsible for fetching package information.
/// - Resolver that runs the PubGrub algorithm and requests package information.
///
/// These components run on separate threads and communicate via channels.
/// The `ResolverState` deduplicates requests sent to remote package source via `OnceMap`.
/// In the algorithm, a package is represented as a `PubGrubPackage`.
/// Compatibility between dependency version requirements and PubGrub version ranges are provided
/// via `SemverPubgrub` layer from `pubgrub-rs/semver-pubgrub`.
///
/// PubGrub works by defining set of ranges of versions of a single package that are either required
/// or disallowed in the solution. Those ranges are used to construct a set of `incompatibilities`,
/// that is a set of requirements that cannot be all satisfied at once by a valid solution (a set of
/// packages that satisfy all incompatibilities cannot is never a solution of the dependency
/// requirements). During the resolver run, new incompatibilities are derived until a valid solution
/// is found in available versions.
pub async fn resolve(
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
        .map_err(|err| format_err!(err))
        .fuse();

    for summary in summaries {
        for dep in summary.full_dependencies() {
            let locked_package_id = lockfile.packages_matching(dep.clone());
            let dep = if let Some(locked_package_id) = locked_package_id {
                rewrite_locked_dependency(dep.clone(), locked_package_id?)
            } else {
                dep.clone()
            };
            if state.index.packages().register(dep.clone()) {
                request_sink.send(Request::Package(dep.clone())).await?;
            }
        }
    }

    let main_package_ids: HashSet<PackageId> =
        HashSet::from_iter(summaries.iter().map(|sum| sum.package_id));

    let (tx, rx) = oneshot::channel();

    let cloned_lockfile = lockfile.clone();
    // Run the resolver in a separate thread.
    // The solution will be sent back to the main thread via the `solution_tx` channel.
    thread::Builder::new()
        .name("scarb-resolver".into())
        .spawn(move || {
            scarb_resolver(state, request_sink, tx, cloned_lockfile, main_package_ids)
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

/// Run dependency resolution with PubGrub algorithm.
fn scarb_resolver(
    state: Arc<ResolverState>,
    request_sink: mpsc::Sender<Request>,
    solution_tx: oneshot::Sender<Result<Resolve, Error>>,
    lockfile: Lockfile,
    main_package_ids: HashSet<PackageId>,
) {
    let result = || {
        let provider =
            PubGrubDependencyProvider::new(main_package_ids, state, request_sink, lockfile);

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

        // Initialize state with main package ids.
        // We add incompatibilities for all other versions of the main packages.
        // This means that the solution must include main package in the provided versions.
        let mut state = State::init(package.clone(), version);
        state
            .unit_propagation(package.clone())
            .map_err(|err| format_err!("unit propagation failed: {:?}", err))?;
        for package_id in rest {
            let package: PubGrubPackage = (*package_id).into();
            let version = package_id.version.clone();
            state.add_incompatibility(Incompatibility::not_root(package.clone(), version.clone()));
            state
                .unit_propagation(package)
                .map_err(|err| format_err!("unit propagation failed: {:?}", err))?
        }

        // Resolve requirements with the PubGrub algorithm.
        let solution =
            pubgrub::resolve_state(&provider, &mut state, package).map_err(format_error)?;

        validate_solution(&solution)?;
        build_resolve(&provider, solution)
    };
    let result = result();
    solution_tx.send(result).unwrap();
}

fn format_error(err: PubGrubError<PubGrubDependencyProvider>) -> Error {
    match err {
        PubGrubError::NoSolution(derivation_tree) => {
            format_err!(
                "version solving failed:\n{}\n",
                DefaultStringReporter::report(&derivation_tree)
            )
        }
        PubGrubError::ErrorChoosingPackageVersion(DependencyProviderError::PackageNotFound {
            name,
            version,
        }) => {
            format_err!("cannot find package `{name} {version}`")
        }
        PubGrubError::ErrorChoosingPackageVersion(DependencyProviderError::PackageQueryFailed(
            err,
        )) => format_err!("{}", err).context("dependency query failed"),
        PubGrubError::ErrorRetrievingDependencies {
            package,
            version,
            source,
        } => {
            Error::from(source).context(format!("cannot get dependencies of `{package}@{version}`"))
        }
        PubGrubError::ErrorInShouldCancel(err) => {
            format_err!("{}", err).context("should cancel failed")
        }
        PubGrubError::Failure(msg) => format_err!("{}", msg).context("resolver failure"),
        PubGrubError::ErrorChoosingPackageVersion(DependencyProviderError::ChannelClosed) => {
            format_err!("channel closed")
        }
    }
}
