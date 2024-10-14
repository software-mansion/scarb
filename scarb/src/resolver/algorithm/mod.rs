use crate::core::lockfile::Lockfile;
use crate::core::registry::Registry;
use crate::core::{ManifestDependency, PackageId, PackageName, Resolve, Summary};
use crate::resolver::algorithm::in_memory_index::{InMemoryIndex, VersionsResponse};
use crate::resolver::algorithm::provider::{
    DependencyProviderError, PubGrubDependencyProvider, PubGrubPackage,
};
use crate::resolver::algorithm::solution::build_resolve;
use anyhow::bail;
use futures::{FutureExt, StreamExt, TryFutureExt};
use indoc::indoc;
use itertools::Itertools;
use pubgrub::error::PubGrubError;
use pubgrub::report::{DefaultStringReporter, Reporter};
use pubgrub::type_aliases::SelectedDependencies;
use pubgrub::{Incompatibility, State};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::thread;
use tokio::runtime::Handle;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::ReceiverStream;

mod in_memory_index;
mod provider;
mod solution;

#[derive(Default)]
struct ResolverState {
    index: InMemoryIndex,
}

impl ResolverState {
    async fn fetch<'a, 'c>(
        self: Arc<Self>,
        provider: Arc<RegistryWrapper<'a>>,
        request_stream: mpsc::Receiver<Request>,
    ) -> Result<(), DependencyProviderError> {
        let mut response_stream = ReceiverStream::new(request_stream)
            .map(|request| self.process_request(request, &*provider).boxed_local())
            // Allow as many futures as possible to start in the background.
            // Backpressure is provided by at a more granular level by `DistributionDatabase`
            // and `SourceDispatch`, as well as the bounded request channel.
            .buffer_unordered(usize::MAX);

        while let Some(response) = response_stream.next().await {
            match response? {
                Some(Response::Package(package, summaries)) => {
                    // dbg!(&summaries);
                    // if summaries.is_empty() {
                    //     continue;
                    // }
                    // let package_name = summaries
                    //     .first()
                    //     .map(|s| s.package_id.name.clone())
                    //     .expect("summaries cannot be empty");
                    self.index
                        .packages()
                        .done(package, Arc::new(VersionsResponse::Found(summaries)));
                }
                None => {}
            }
        }
        Ok(())
    }

    async fn process_request<'a>(
        &self,
        request: Request,
        registry: &RegistryWrapper<'a>,
    ) -> Result<Option<Response>, DependencyProviderError> {
        match request {
            Request::Package(package) => {
                self.index.packages().register(package.clone());
                let dependency: ManifestDependency = (&package).into();
                let summaries = registry.registry.query(&dependency).await?;
                Ok(Some(Response::Package(dbg!(package), summaries)))
            }
        }
    }
}

pub struct RegistryWrapper<'a> {
    registry: &'a dyn Registry,
}

#[derive(Debug)]
pub(crate) enum Request {
    Package(PubGrubPackage),
}

pub(crate) enum Response {
    Package(PubGrubPackage, Vec<Summary>),
}

#[allow(clippy::dbg_macro)]
#[allow(dead_code)]
pub async fn resolve<'c>(
    summaries: &[Summary],
    registry: &dyn Registry,
    _lockfile: Lockfile,
    _handle: &'c Handle,
) -> anyhow::Result<Resolve> {
    let state = Arc::new(ResolverState::default());

    let (request_sink, request_stream): (mpsc::Sender<Request>, mpsc::Receiver<Request>) =
        mpsc::channel(300);

    let registry_wrapper = Arc::new(RegistryWrapper { registry });

    let requests_fut = state
        .clone()
        .fetch(registry_wrapper.clone(), request_stream)
        .map_err(|err| anyhow::format_err!(err))
        .fuse();

    for summary in summaries {
        let package: PubGrubPackage = summary.package_id.into();
        if state.index.packages().register(package.clone()) {
            request_sink.send(dbg!(Request::Package(package))).await?;
        }
        for dep in summary.full_dependencies() {
            let package: PubGrubPackage = dep.into();
            if state.index.packages().register(package.clone()) {
                request_sink.send(dbg!(Request::Package(package))).await?;
            }
        }
    }

    let main_package_ids: HashSet<PackageId> =
        HashSet::from_iter(summaries.iter().map(|sum| sum.package_id));

    let (tx, rx) = oneshot::channel();

    thread::Builder::new()
        .name("scarb-resolver".into())
        .spawn(move || {
            let result = || {
                let provider =
                    PubGrubDependencyProvider::new(main_package_ids, state, request_sink);

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

                dbg!(&solution);

                validate_solution(&solution)?;
                build_resolve(&provider, solution)
            };
            let result = result();
            tx.send(result).unwrap();
        })?;

    let resolve_fut = async move {
        rx.await
            // .map_err(|_| (ResolveError::ChannelClosed, FxHashSet::default()))
            // .map_err(|_| DependencyProviderError::ChannelClosed)
            .map_err(|err| anyhow::format_err!("channel closed"))
            .and_then(|result| result)
    };

    // match tokio::try_join!(requests_fut, resolve_fut) {
    //     Ok(((), resolution)) => {
    //         // state.on_complete();
    //         Ok(resolution)
    //     }
    //     Err(err) => Err(err).context("resolver failed"),
    // }
    let res = tokio::try_join!(requests_fut, resolve_fut)?;
    Ok(res.1)
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

fn validate_solution(
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
