use crate::core::lockfile::Lockfile;
use crate::core::registry::Registry;
use crate::core::registry::patch_map::PatchMap;
use crate::core::{PackageId, Resolve, Summary};
use crate::resolver::provider::{
    DependencyProviderError, PubGrubDependencyProvider, PubGrubPackage, lock_dependency,
};
use crate::resolver::solution::{build_resolve, validate_solution};
use crate::resolver::state::{Request, ResolverState};
use anyhow::{Error, bail, format_err};
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
/// # Arguments
///
/// * `summaries` - the list of all top-level packages that are intended to be part of
///   the lock file (resolve output).
///   These typically are a list of all workspace members.
///
/// * `registry` - this is the source from which all package summaries are loaded.
///   It is expected that this is extensively configured ahead of time and is idempotent with
///   our requests to it (aka returns the same results for the same query every time).
///   It is also advised to implement internal caching, as the resolver may frequently ask
///   repetitive queries.
///
/// * `lockfile` - a [`Lockfile`] instance, which is used to guide the resolution process. Empty
///   lockfile will result in no guidance. This function does not read or write lock files from
///   the filesystem.
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
#[tracing::instrument(level = "trace", skip_all)]
pub async fn resolve(
    summaries: &[Summary],
    registry: &dyn Registry,
    patch_map: &PatchMap,
    lockfile: Lockfile,
    require_audits: bool,
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
            let dep = patch_map.lookup(dep);
            let dep = lock_dependency(&lockfile, dep.clone())?;
            if state.index.packages().register(dep.clone().into()) {
                request_sink.send(Request::Package(dep.clone())).await?;
            }
        }
    }

    let main_package_ids: HashSet<PackageId> =
        HashSet::from_iter(summaries.iter().map(|sum| sum.package_id));

    let (tx, rx) = oneshot::channel();

    let cloned_lockfile = lockfile.clone();
    let cloned_patch_map = patch_map.clone();
    // Run the resolver in a separate thread.
    // The solution will be sent back to the main thread via the `solution_tx` channel.
    thread::Builder::new()
        .name("scarb-resolver".into())
        .spawn(move || {
            scarb_resolver(
                state,
                request_sink,
                tx,
                cloned_patch_map,
                cloned_lockfile,
                main_package_ids,
                require_audits,
            )
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
    patch_map: PatchMap,
    lockfile: Lockfile,
    main_package_ids: HashSet<PackageId>,
    require_audits: bool,
) {
    let result = || {
        let provider = PubGrubDependencyProvider::new(
            main_package_ids,
            state,
            request_sink,
            patch_map,
            lockfile,
            require_audits,
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

#[cfg(test)]
mod tests {
    //! These tests largely come from Elixir's `hex_solver` test suite.

    use anyhow::Result;
    use indoc::indoc;
    use itertools::Itertools;
    use semver::Version;
    use similar_asserts::assert_serde_eq;
    use tokio::runtime::Builder;

    use crate::core::lockfile::{Lockfile, PackageLock};
    use crate::core::package::PackageName;
    use crate::core::registry::mock::{MockRegistry, deps, locks, pkgs, registry};
    use crate::core::registry::patch_map::PatchMap;
    use crate::core::{ManifestDependency, PackageId, Resolve, SourceId, TargetKind};

    fn check(
        registry: MockRegistry,
        roots: &[&[ManifestDependency]],
        expected: Result<&[PackageId], &str>,
    ) {
        check_with_lock(registry, roots, &[], expected)
    }

    fn check_with_lock(
        registry: MockRegistry,
        roots: &[&[ManifestDependency]],
        locks: &[PackageLock],
        expected: Result<&[PackageId], &str>,
    ) {
        let root_ids = (1..).map(|n| package_id(format!("root_{n}")));

        let roots = roots
            .iter()
            .zip(root_ids)
            .map(|(&deps, pid)| (deps, pid))
            .collect_vec();

        let resolve = resolve_with_lock(registry, roots, locks);

        let resolve = resolve
            .map(|r| {
                r.graph
                    .nodes()
                    .filter(|id| {
                        !id.name.as_str().starts_with("root_")
                            && id.name != PackageName::CORE
                            && id.name != PackageName::TEST_PLUGIN
                    })
                    .sorted()
                    .collect_vec()
            })
            .map_err(|e| e.to_string());

        let resolve = match resolve {
            Ok(ref v) => Ok(v.as_slice()),
            Err(ref e) => Err(e.as_str()),
        };

        assert_serde_eq!(expected, resolve);
    }

    fn resolve(
        registry: MockRegistry,
        roots: Vec<(&[ManifestDependency], PackageId)>,
    ) -> Result<Resolve> {
        resolve_with_lock(registry, roots, &[])
    }

    fn resolve_with_lock(
        mut registry: MockRegistry,
        roots: Vec<(&[ManifestDependency], PackageId)>,
        locks: &[PackageLock],
    ) -> Result<Resolve> {
        let runtime = Builder::new_multi_thread().build().unwrap();

        let summaries = roots
            .iter()
            .map(|(deps, package_id)| {
                registry.put(*package_id, deps.to_vec());
                registry
                    .get_package(*package_id)
                    .unwrap()
                    .manifest
                    .summary
                    .clone()
            })
            .collect_vec();

        let lockfile = Lockfile::new(locks.iter().cloned());
        let patch_map = PatchMap::new();
        let require_audits = false;
        runtime.block_on(super::resolve(
            &summaries,
            &registry,
            &patch_map,
            lockfile,
            require_audits,
        ))
    }

    fn package_id<S: AsRef<str>>(name: S) -> PackageId {
        let name = PackageName::new(name.as_ref());
        PackageId::new(name, Version::new(1, 0, 0), SourceId::mock_path())
    }

    #[test]
    fn no_input() {
        check(registry![], &[deps![]], Ok(pkgs![]))
    }

    #[test]
    fn single_fixed_dep() {
        check(
            registry![("foo v1.0.0-rc.0", []),],
            &[deps![("foo", "=1.0.0-rc.0")]],
            Ok(pkgs!["foo v1.0.0-rc.0"]),
        )
    }

    #[test]
    fn single_caret_dep() {
        check(
            registry![("foo v1.0.0", []),],
            &[deps![("foo", "1.0.0")]],
            Ok(pkgs!["foo v1.0.0"]),
        )
    }

    #[test]
    fn single_fixed_dep_with_multiple_versions() {
        check(
            registry![("foo v1.1.0", []), ("foo v1.0.0", []),],
            &[deps![("foo", "=1.0.0")]],
            Ok(pkgs!["foo v1.0.0"]),
        )
    }

    #[test]
    fn single_caret_dep_with_multiple_versions() {
        check(
            registry![("foo v1.1.0", []), ("foo v1.0.0", []),],
            &[deps![("foo", "1.0.0")]],
            Ok(pkgs!["foo v1.1.0"]),
        )
    }

    #[test]
    fn single_tilde_dep_with_multiple_versions() {
        check(
            registry![("foo v1.1.0", []), ("foo v1.0.0", []),],
            &[deps![("foo", "~1.0.0")]],
            Ok(pkgs!["foo v1.0.0"]),
        )
    }

    #[test]
    fn single_older_dep_with_dependency_and_multiple_versions() {
        check(
            registry![
                ("foo v1.1.0", []),
                ("foo v1.0.0", [("bar", "=1.0.0")]),
                ("bar v1.0.0", []),
            ],
            &[deps![("foo", "<1.1.0")]],
            Ok(pkgs!["bar v1.0.0", "foo v1.0.0"]),
        )
    }

    #[test]
    fn single_newer_dep_without_dependency_and_multiple_versions() {
        check(
            registry![
                ("foo v1.1.0", []),
                ("foo v1.0.0", [("bar", "=1.0.0")]),
                ("bar v1.0.0", []),
            ],
            &[deps![("foo", "1.1.0")]],
            Ok(pkgs!["foo v1.1.0"]),
        )
    }

    #[test]
    fn prioritize_stable_versions() {
        check(
            registry![
                ("foo v1.0.0", []),
                ("foo v1.1.0", []),
                ("foo v1.2.0-dev", []),
            ],
            &[deps![("foo", "1.1.0")]],
            Ok(pkgs!["foo v1.1.0"]),
        )
    }

    #[test]
    fn two_deps() {
        check(
            registry![("foo v1.0.0", []), ("bar v2.0.0", []),],
            &[deps![("foo", "1")], deps![("bar", "2")]],
            Ok(pkgs!["bar v2.0.0", "foo v1.0.0"]),
        )
    }

    #[test]
    fn nested_deps() {
        check(
            registry![("foo v1.0.0", [("bar", "1.0.0")]), ("bar v1.0.0", []),],
            &[deps![("foo", "1.0")]],
            Ok(pkgs!["bar v1.0.0", "foo v1.0.0"]),
        )
    }

    #[test]
    fn backtrack_1() {
        check(
            registry![
                ("foo v2.0.0", [("bar", "2.0.0"), ("baz", "1.0.0")]),
                ("foo v1.0.0", [("bar", "1.0.0")]),
                ("bar v2.0.0", [("baz", "2.0.0")]),
                ("bar v1.0.0", [("baz", "1.0.0")]),
                ("baz v2.0.0", []),
                ("baz v1.0.0", []),
            ],
            &[deps![("foo", "*")]],
            Ok(pkgs!["bar v1.0.0", "baz v1.0.0", "foo v1.0.0"]),
        )
    }

    #[test]
    fn backtrack_2() {
        check(
            registry![
                ("foo v2.6.0", [("baz", "~1.7.0")]),
                ("foo v2.7.0", [("baz", "~1.7.1")]),
                ("foo v2.8.0", [("baz", "~1.7.1")]),
                ("foo v2.9.0", [("baz", "1.8.0")]),
                ("bar v1.1.1", [("baz", ">= 1.7.0")]),
                ("baz v1.7.0", []),
                ("baz v1.7.1", []),
                ("baz v1.8.0", []),
                ("baz v2.1.0", []),
            ],
            &[deps![("bar", "~1.1.0"), ("foo", "~2.7")]],
            Ok(pkgs!["bar v1.1.1", "baz v1.7.1", "foo v2.7.0"]),
        )
    }

    #[test]
    #[ignore = "does not work as expected"]
    fn overlapping_ranges() {
        check(
            registry![
                ("foo v1.0.0", [("bar", "*")]),
                ("foo v1.1.0", [("bar", "2")]),
                ("bar v1.0.0", []),
            ],
            &[deps![("foo", "1")]],
            Ok(pkgs!["bar v1.0.0", "foo v1.0.0"]),
        )
    }

    #[test]
    fn cycle() {
        check(
            registry![
                ("foo v1.0.0", [("bar", "2.0.0")]),
                ("bar v2.0.0", [("foo", "1.0.0")]),
            ],
            &[deps![("foo", "1")]],
            Ok(pkgs!["bar v2.0.0", "foo v1.0.0"]),
        )
    }

    #[test]
    fn sub_dependencies() {
        check(
            registry![
                ("foo v1.0.0", []),
                ("foo v2.0.0", []),
                ("top1 v1.0.0", [("foo", "1.0.0")]),
                ("top2 v1.0.0", [("foo", "2.0.0")]),
            ],
            &[deps![("top1", "1"), ("top2", "1")]],
            Err(indoc! {"
                version solving failed:
                Because there is no version of top2 in >1.0.0, <2.0.0 and top2 1.0.0 depends on foo >=2.0.0, <3.0.0, top2 >=1.0.0, <2.0.0 depends on foo >=2.0.0, <3.0.0.
                And because top1 1.0.0 depends on foo >=1.0.0, <2.0.0 and there is no version of top1 in >1.0.0, <2.0.0, top1 >=1.0.0, <2.0.0, top2 >=1.0.0, <2.0.0 are incompatible.
                And because root_1 1.0.0 depends on top1 >=1.0.0, <2.0.0 and root_1 1.0.0 depends on top2 >=1.0.0, <2.0.0, root_1 1.0.0 is forbidden.
            "}),
        )
    }

    #[test]
    fn missing_dependency() {
        check(
            registry![],
            &[deps![("foo", "1.0.0")]],
            Err(r#"MockRegistry/query: cannot find foo ^1.0.0"#),
        )
    }

    #[test]
    fn unsatisfied_version_constraint() {
        check(
            registry![("foo v2.0.0", []),],
            &[deps![("foo", "1.0.0")]],
            Err(r#"cannot get dependencies of `root_1@1.0.0`"#),
        )
    }

    #[test]
    fn unsatisfied_source_constraint() {
        check(
            registry![("foo v1.0.0", []),],
            &[deps![("foo", "1.0.0", "git+https://example.git/foo.git")]],
            Err(r#"MockRegistry/query: cannot find foo ^1.0.0 (git+https://example.git/foo.git)"#),
        )
    }

    #[test]
    fn no_matching_transient_dependency_1() {
        check(
            registry![
                ("a v3.9.4", [("b", "3.9.4")]),
                ("a v3.9.5", [("b", "3.9.5")]),
                ("a v3.9.8", [("b", "3.9.8")]),
                ("b v3.8.5-rc.2", []),
                ("b v3.8.5", []),
                ("b v3.8.14", []),
            ],
            &[deps![("a", "~3.6"), ("b", "~3.6")]],
            Err(r#"cannot get dependencies of `root_1@1.0.0`"#),
        )
    }

    #[test]
    fn no_matching_transient_dependency_2() {
        check(
            registry![
                ("a v3.8.10", [("b", "3.8.10")]),
                ("a v3.8.11", [("b", "3.8.11")]),
                ("a v3.8.14", [("b", "3.8.14")]),
                ("a v3.8.25", [("b", "3.8.25")]),
                ("c v1.1.0", [("d", "~2.8.0")]),
                ("d v2.8.3", []),
                ("b v3.8.14", [("d", "2.11.0")]),
                ("b v3.8.25", [("d", "3.1.0")]),
                ("b v3.8.5-rc.2", [("d", "2.9.0")]),
                ("b v3.8.5", [("d", "2.9.0")]),
            ],
            &[deps![("a", "~3.6"), ("c", "~1.1"), ("b", "~3.6")]],
            Err(r#"cannot get dependencies of `root_1@1.0.0`"#),
        )
    }

    #[test]
    fn no_matching_transient_dependency_3() {
        check(
            registry![
                ("a v3.8.25", [("b", "3.8.25")]),
                ("b v3.8.12-rc.3", [("d", "2.11.0")]),
                ("a v3.8.21", [("b", "3.8.21")]),
                ("b v3.8.19", [("d", "3.1.0")]),
                ("b v3.8.25", [("d", "3.1.0")]),
                ("e v1.6.0", [("a", "~3.8.0")]),
                ("b v3.8.14", [("d", "2.11.0")]),
                ("a v3.9.8", [("b", "3.9.8")]),
                ("a v3.8.5", [("b", "3.8.5")]),
                (
                    "e v1.3.2",
                    [("a", "~3.7.11"), ("d", "~2.9"), ("b", "~3.7.11")]
                ),
            ],
            &[deps![("e", "~1.0"), ("a", "~3.7"), ("b", "~3.7")]],
            Err(r#"cannot get dependencies of `root_1@1.0.0`"#),
        )
    }

    #[test]
    fn can_add_target_kind_dep() {
        check(
            registry![("foo v1.0.0", []), ("boo v1.0.0", [])],
            &[deps![
                ("foo", "1.0.0", (), "test"),
                ("foo", "1.0.0", (), "dojo"),
                ("boo", "1.0.0")
            ]],
            Ok(pkgs!["boo v1.0.0", "foo v1.0.0"]),
        );
    }

    #[test]
    fn can_resolve_target_kind_dep() {
        let root = package_id("bar");
        let resolve = resolve(
            registry![("foo v1.0.0", []), ("boo v1.0.0", [])],
            vec![(
                deps![
                    ("foo", "1.0.0", (), "test"),
                    ("foo", "1.0.0", (), "dojo"),
                    ("boo", "1.0.0")
                ],
                root,
            )],
        )
        .unwrap();

        let mut test_solution = resolve.solution_of(root, &TargetKind::TEST);
        test_solution.sort();
        assert_eq!(test_solution.len(), 4);
        assert_eq!(
            test_solution
                .iter()
                .map(|p| p.name.clone().to_string())
                .collect_vec(),
            vec!["bar", "boo", "core", "foo"],
        );

        let mut lib_solution = resolve.solution_of(root, &TargetKind::LIB);
        lib_solution.sort();
        assert_eq!(lib_solution.len(), 3);
        assert_eq!(
            lib_solution
                .into_iter()
                .map(|p| p.name.clone().to_string())
                .collect_vec(),
            vec!["bar", "boo", "core"],
        );
    }

    #[test]
    fn lock_dependency() {
        check_with_lock(
            registry![("foo v1.0.0", []), ("foo v1.0.1", []), ("boo v1.0.0", [])],
            &[deps![
                ("foo", ">=1.0.1", (), "test"),
                ("foo", ">=1.0.1", (), "dojo"),
                ("boo", "1.0.0")
            ]],
            locks![("foo v1.0.1", ["bar"])],
            Ok(pkgs!["boo v1.0.0", "foo v1.0.1"]),
        );
    }

    #[test]
    fn lock_dependency_with_git() {
        check_with_lock(
            registry![
                (
                    "foo v1.0.0",
                    [("baz", "1.0.0", "git+https://example.com/baz.git")]
                ),
                (
                    "bar v1.0.0",
                    [("baz", "1.0.0", "git+https://example.com/baz.git")]
                ),
                ("baz v1.0.0 (git+https://example.com/baz.git)", []),
                ("baz v1.0.0 (git+https://example.com/baz.git#some_rev)", []),
            ],
            &[deps![("foo", "1.0.0"), ("bar", "1.0.0")]],
            locks![("baz v1.0.0 (git+https://example.com/baz.git#some_rev)", [])],
            Ok(pkgs![
                "bar v1.0.0",
                "baz v1.0.0 (git+https://example.com/baz.git#some_rev)",
                "foo v1.0.0"
            ]),
        )
    }

    #[test]
    fn lock_conflict_1() {
        check_with_lock(
            registry![("foo v1.0.0", []),],
            &[deps![("foo", "2.0.0"),]],
            locks![("foo v1.0.0", [])],
            Err("cannot get dependencies of `root_1@1.0.0`"),
        );
    }

    #[test]
    fn lock_conflict_2() {
        check_with_lock(
            registry![("foo v1.0.0", []),],
            &[deps![("foo", "1.0.0"),]],
            locks![("foo v2.0.0", [])],
            Ok(pkgs!["foo v1.0.0"]),
        );
    }

    #[test]
    fn lock_downgrade() {
        check_with_lock(
            registry![
                ("foo v1.0.0", []),
                ("foo v1.1.0", []),
                ("foo v1.2.0", []),
                ("boo v1.0.0", [])
            ],
            &[deps![("boo", "1.0.0"), ("foo", "~1"),]],
            locks![("foo v1.1.0", [])],
            Ok(pkgs!["boo v1.0.0", "foo v1.1.0"]),
        );
    }

    #[test]
    fn lock_matching_only_some_deps_is_upgraded() {
        check_with_lock(
            registry![("foo v0.1.0", []), ("foo v0.1.1", [])],
            // First dep matches: >=0.1.0 && < 0.2.0
            // Second dep matches: >=0.1.1 && < 0.2.0
            // So they clearly intersect at v0.1.1
            &[deps![("foo", "^0.1.0"),], deps![("foo", "^0.1.1")]],
            // However, the project locks foo to 0.1.0 (e.g. both deps set to =0.1.0 in previous run).
            // Which can lock ^0.1.0, but cannot lock ^0.1.1
            locks![("foo v0.1.0", [])],
            // If we run the resolver with both deps set to ^0.1.0,
            // it would resolve to 0.1.0 keeping the lock.
            // If we run the resolver with both deps set to ^0.1.1,
            // it would resolve to 0.1.1, upgrading the lock, as not matching deps.
            // Unfortunately, with this configuration, the lock will be kept and resolution will fail
            // with the following error.
            // Ideally, the lock should be upgraded instead.
            Ok(pkgs!["foo v0.1.1"]),
        );
    }

    #[test]
    #[ignore = "optional deps are not implemented yet"]
    fn skip_single_optional() {
        //     Registry.put("foo", "1.0.0", [])
        //
        //     assert run([{"foo", "1.0.0", optional: true}]) == %{}
    }

    #[test]
    #[ignore = "optional deps are not implemented yet"]
    fn skip_locked_optional() {
        //     Registry.put("foo", "1.0.0", [])
        //
        //     assert run([{"foo", "1.0.0", optional: true}], [{"foo", "1.0.0"}]) == %{}
    }

    #[test]
    #[ignore = "optional deps are not implemented yet"]
    fn skip_conflicting_optionals() {
        //     Registry.put("foo", "1.0.0", [{"bar", "1.0.0"}, {"car", "~1.0", optional: true}])
        //     Registry.put("bar", "1.0.0", [{"car", "~2.0", optional: true}])
        //     Registry.put("car", "1.0.0", [])
        //     Registry.put("car", "2.0.0", [])
        //
        //     assert run([{"foo", "1.0.0"}], []) == %{
        //              "foo" => "1.0.0",
        //              "bar" => "1.0.0"
        //            }
    }

    #[test]
    #[ignore = "optional deps are not implemented yet"]
    fn skip_transitive_optionals() {
        //     # car's fuse dependency needs to be a subset of bar's fuse dependency
        //     # fuse 1.0.0 ⊃ fuse ~1.0
        //
        //     Registry.put("foo", "1.0.0", [{"bar", "1.0.0"}, {"car", "1.0.0"}])
        //     Registry.put("bar", "1.0.0", [{"fuse", "~1.0", optional: true}])
        //     Registry.put("car", "1.0.0", [{"fuse", "1.0.0", optional: true}])
        //     Registry.put("fuse", "1.0.0", [])
        //
        //     assert run([{"foo", "1.0.0"}], []) == %{
        //              "foo" => "1.0.0",
        //              "bar" => "1.0.0",
        //              "car" => "1.0.0"
        //            }
    }

    #[test]
    #[ignore = "optional deps are not implemented yet"]
    fn skip_conflicting_transitive_optionals() {
        //     Registry.put("foo", "1.0.0", [{"bar", "1.0.0"}, {"car", "1.0.0"}])
        //     Registry.put("bar", "1.0.0", [{"fuse", "~1.0", optional: true}])
        //     Registry.put("car", "1.0.0", [{"fuse", "~2.0", optional: true}])
        //     Registry.put("fuse", "1.0.0", [])
        //     Registry.put("fuse", "2.0.0", [])
        //
        //     assert run([{"foo", "1.0.0"}], []) == %{
        //              "foo" => "1.0.0",
        //              "bar" => "1.0.0",
        //              "car" => "1.0.0"
        //            }
    }

    #[test]
    #[ignore = "optional deps are not implemented yet"]
    fn locked_optional_does_not_conflict() {
        //     Registry.put("foo", "1.0.0", [])
        //
        //     assert run([{"foo", "1.0.0", optional: true}], [{"foo", "1.1.0"}]) == %{}
    }

    #[test]
    #[ignore = "optional deps are not implemented yet"]
    fn skip_optional_with_backtrack() {
        //     Registry.put("foo", "1.1.0", [{"bar", "1.1.0"}, {"baz", "1.0.0"}, {"opt", "1.0.0"}])
        //     Registry.put("foo", "1.0.0", [{"bar", "1.0.0"}, {"opt", "1.0.0", optional: true}])
        //     Registry.put("bar", "1.1.0", [{"baz", "1.1.0"}, {"opt", "1.0.0"}])
        //     Registry.put("bar", "1.0.0", [{"baz", "1.0.0"}, {"opt", "1.0.0", optional: true}])
        //     Registry.put("baz", "1.1.0", [{"opt", "1.0.0"}])
        //     Registry.put("baz", "1.0.0", [{"opt", "1.0.0", optional: true}])
        //     Registry.put("opt", "1.0.0", [])
        //
        //     assert run([{"foo", "~1.0"}]) == %{"foo" => "1.0.0", "bar" => "1.0.0", "baz" => "1.0.0"}
    }

    #[test]
    #[ignore = "optional deps are not implemented yet"]
    fn select_optional() {
        //     Registry.put("foo", "1.0.0", [])
        //     Registry.put("bar", "1.0.0", [{"foo", "1.0.0"}])
        //
        //     assert run([{"foo", "1.0.0", optional: true}, {"bar", "1.0.0"}]) == %{
        //              "foo" => "1.0.0",
        //              "bar" => "1.0.0"
        //            }
    }

    #[test]
    #[ignore = "optional deps are not implemented yet"]
    fn select_older_optional() {
        //     Registry.put("foo", "1.0.0", [])
        //     Registry.put("foo", "1.1.0", [])
        //     Registry.put("bar", "1.0.0", [{"foo", "~1.0"}])
        //
        //     assert run([{"foo", "~1.0.0", optional: true}, {"bar", "1.0.0"}]) == %{
        //              "foo" => "1.0.0",
        //              "bar" => "1.0.0"
        //            }
    }

    #[test]
    #[ignore = "optional deps are not implemented yet"]
    fn select_optional_with_backtrack() {
        //     Registry.put("foo", "1.1.0", [{"bar", "1.1.0"}, {"baz", "1.0.0"}, {"opt", "1.0.0"}])
        //     Registry.put("foo", "1.0.0", [{"bar", "1.0.0"}, {"opt", "1.0.0", optional: true}])
        //     Registry.put("bar", "1.1.0", [{"baz", "1.1.0"}, {"opt", "1.0.0"}])
        //     Registry.put("bar", "1.0.0", [{"baz", "1.0.0"}, {"opt", "1.0.0", optional: true}])
        //     Registry.put("baz", "1.1.0", [{"opt", "1.0.0", optional: true}])
        //     Registry.put("baz", "1.0.0", [{"opt", "1.0.0"}])
        //     Registry.put("opt", "1.0.0", [])
        //
        //     assert run([{"foo", "~1.0"}]) == %{
        //              "foo" => "1.0.0",
        //              "bar" => "1.0.0",
        //              "baz" => "1.0.0",
        //              "opt" => "1.0.0"
        //            }
    }

    #[test]
    #[ignore = "optional deps are not implemented yet"]
    fn optional_with_conflict() {
        //     Registry.put("foo", "1.0.0", [{"bar", "~2.0", optional: true}])
        //     Registry.put("baz", "1.0.0", [{"bar", "~1.0"}])
        //     Registry.put("car", "1.0.0", [{"foo", ">= 1.0.0"}])
        //     Registry.put("bar", "1.0.0", [])
        //     Registry.put("bar", "2.0.0", [])
        //
        //     assert {:conflict, _, _} = run([{"car", ">= 0.0.0"}, {"baz", ">= 0.0.0"}])
    }

    #[test]
    #[ignore = "overrides are not implemented yet"]
    fn ignores_incompatible_constraint() {
        //     Registry.put("foo", "1.0.0", [{"bar", "2.0.0"}])
        //     Registry.put("bar", "1.0.0", [])
        //     Registry.put("bar", "2.0.0", [])
        //
        //     assert run([{"foo", "1.0.0"}, {"bar", "1.0.0"}], [], ["bar"]) == %{
        //              "foo" => "1.0.0",
        //              "bar" => "1.0.0"
        //            }
    }

    #[test]
    #[ignore = "overrides are not implemented yet"]
    fn ignores_compatible_constraint() {
        //     Registry.put("foo", "1.0.0", [{"bar", "~1.0.0"}])
        //     Registry.put("bar", "1.0.0", [])
        //     Registry.put("bar", "1.1.0", [])
        //
        //     assert run([{"foo", "1.0.0"}, {"bar", "~1.0"}], [], ["bar"]) == %{
        //              "foo" => "1.0.0",
        //              "bar" => "1.1.0"
        //            }
    }

    #[test]
    #[ignore = "overrides are not implemented yet"]
    fn skips_overridden_dependency_outside_of_the_root() {
        //     Registry.put("foo", "1.0.0", [{"bar", "1.0.0"}])
        //     Registry.put("bar", "1.0.0", [{"baz", "1.0.0"}])
        //     Registry.put("baz", "1.0.0", [])
        //
        //     assert run([{"foo", "1.0.0"}], [], ["baz"]) == %{
        //              "foo" => "1.0.0",
        //              "bar" => "1.0.0"
        //            }
    }

    #[test]
    #[ignore = "overrides are not implemented yet"]
    fn do_not_skip_overridden_dependency_outside_of_the_root_when_label_does_not_match() {
        //     Registry.put("foo", "1.0.0", [{"bar", "1.0.0"}])
        //     Registry.put("bar", "1.0.0", [{"baz", "1.0.0", label: "not-baz"}])
        //     Registry.put("baz", "1.0.0", [])
        //
        //     assert run([{"foo", "1.0.0"}], [], ["baz"]) == %{
        //              "foo" => "1.0.0",
        //              "bar" => "1.0.0",
        //              "baz" => "1.0.0"
        //            }
    }

    #[test]
    #[ignore = "overrides are not implemented yet"]
    fn overridden_dependencies_does_not_unlock() {
        //     Registry.put("foo", "1.0.0", [])
        //     Registry.put("foo", "1.1.0", [])
        //
        //     assert run([{"foo", "~1.0"}], [{"foo", "1.0.0"}], ["foo"]) == %{"foo" => "1.0.0"}
    }

    #[test]
    fn mixed_sources() {
        check(
            registry![
                (
                    "foo v1.0.0",
                    [("baz", "1.0.0", "git+https://example.com/baz.git")]
                ),
                (
                    "bar v1.0.0",
                    [("baz", "1.0.0", "git+https://example.com/baz.git")]
                ),
                ("baz v1.0.0 (git+https://example.com/baz.git)", []),
            ],
            &[deps![("foo", "1.0.0"), ("bar", "1.0.0")]],
            Ok(pkgs![
                "bar v1.0.0",
                "baz v1.0.0 (git+https://example.com/baz.git)",
                "foo v1.0.0"
            ]),
        )
    }

    #[test]
    fn source_conflict() {
        check(
            registry![
                (
                    "foo v1.0.0",
                    [("baz", "1.0.0", "git+https://example.com/foo.git")]
                ),
                (
                    "bar v1.0.0",
                    [("baz", "1.0.0", "git+https://example.com/bar.git")]
                ),
                ("baz v1.0.0 (git+https://example.com/foo.git)", []),
                ("baz v1.0.0 (git+https://example.com/bar.git)", []),
            ],
            &[deps![("foo", "1.0.0"), ("bar", "1.0.0")]],
            Err(indoc! {"
                found dependencies on the same package `baz` coming from \
                incompatible sources:
                source 1: git+https://example.com/bar.git
                source 2: git+https://example.com/foo.git
            "}),
        )
    }
}
