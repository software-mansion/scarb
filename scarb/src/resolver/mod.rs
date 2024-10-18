use anyhow::Result;

use crate::core::lockfile::Lockfile;
use crate::core::registry::Registry;
use crate::core::resolver::Resolve;
use crate::core::Summary;

mod algorithm;
mod primitive;

/// Builds the list of all packages required to build the first argument.
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
/// * `ui` - an [`Ui`] instance used to show warnings to the user.
#[tracing::instrument(level = "trace", skip_all)]
pub async fn resolve(
    summaries: &[Summary],
    registry: &dyn Registry,
    lockfile: Lockfile,
) -> Result<Resolve> {
    primitive::resolve(summaries, registry, lockfile).await
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
    use crate::core::registry::mock::{deps, locks, pkgs, registry, MockRegistry};
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
        runtime.block_on(super::resolve(&summaries, &registry, lockfile))
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
            registry![("foo v1.0.0", []),],
            &[deps![("foo", "=1.0.0")]],
            Ok(pkgs!["foo v1.0.0"]),
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
            // TODO(#2): Expected result is commented out.
            // Ok(pkgs![
            //     "bar v1.0.0",
            //     "baz v1.0.0",
            //     "foo v1.0.0"
            // ]),
            Err(indoc! {"
            Version solving failed:
            - bar v2.0.0 cannot use baz v1.0.0, because bar requires baz ^2.0.0

            Scarb does not have real version solving algorithm yet.
            Perhaps in the future this conflict could be resolved, but currently,
            please upgrade your dependencies to use latest versions of their dependencies.
            "}),
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
            // TODO(#2): Expected result is commented out.
            // Ok(pkgs![
            //     "bar v1.1.1",
            //     "baz v1.7.1",
            //     "foo v2.7.0"
            // ]),
            Err(indoc! {"
            Version solving failed:
            - foo v2.7.0 cannot use baz v2.1.0, because foo requires baz ~1.7.1

            Scarb does not have real version solving algorithm yet.
            Perhaps in the future this conflict could be resolved, but currently,
            please upgrade your dependencies to use latest versions of their dependencies.
            "}),
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
            Version solving failed:
            - top2 v1.0.0 cannot use foo v1.0.0, because top2 requires foo ^2.0.0

            Scarb does not have real version solving algorithm yet.
            Perhaps in the future this conflict could be resolved, but currently,
            please upgrade your dependencies to use latest versions of their dependencies.
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
            Err(r#"cannot find package foo"#),
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
            Err(r#"cannot find package a"#),
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
            Err(r#"cannot find package a"#),
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
            Err(r#"cannot find package e"#),
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
            Err("cannot find package foo"),
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
                source 1: git+https://example.com/foo.git
                source 2: git+https://example.com/bar.git
            "}),
        )
    }
}
