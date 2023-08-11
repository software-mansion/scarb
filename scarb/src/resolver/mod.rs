use std::collections::HashMap;

use anyhow::{bail, Result};
use indoc::{formatdoc, indoc};
use petgraph::graphmap::DiGraphMap;

use crate::core::registry::Registry;
use crate::core::resolver::{DependencyEdge, Resolve};
use crate::core::{DepKind, ManifestDependency, PackageId, Summary, TargetKind};

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
#[tracing::instrument(level = "trace", skip_all)]
pub async fn resolve(summaries: &[Summary], registry: &dyn Registry) -> Result<Resolve> {
    // TODO(#2): This is very bad, use PubGrub here.
    let mut graph = DiGraphMap::<PackageId, DependencyEdge>::new();

    let mut packages: HashMap<_, _> = HashMap::from_iter(
        summaries
            .iter()
            .map(|s| (s.package_id.name.clone(), s.package_id)),
    );

    let mut summaries: HashMap<_, _> = summaries
        .iter()
        .map(|s| (s.package_id, s.clone()))
        .collect();

    let mut queue: Vec<PackageId> = summaries.keys().copied().collect();
    while !queue.is_empty() {
        let mut next_queue = Vec::new();

        for package_id in queue {
            graph.add_node(package_id);

            for dep in summaries[&package_id].clone().full_dependencies() {
                let dep = rewrite_dependency_source_id(registry, &package_id, dep).await?;

                let results = registry.query(&dep).await?;

                let Some(dep_summary) = results.first() else {
                    bail!("cannot find package {}", dep.name)
                };

                let dep_target_kind: Option<TargetKind> = match dep.kind.clone() {
                    DepKind::Normal => None,
                    DepKind::Target(target_kind) => Some(target_kind),
                };
                let dep = dep_summary.package_id;

                if let Some(existing) = packages.get(dep.name.as_ref()) {
                    if existing.source_id != dep.source_id {
                        bail!(
                            indoc! {"
                            found dependencies on the same package `{}` coming from incompatible \
                            sources:
                            source 1: {}
                            source 2: {}
                            "},
                            dep.name,
                            existing.source_id,
                            dep.source_id
                        );
                    }
                }

                let weight = graph
                    .edge_weight(package_id, dep)
                    .cloned()
                    .unwrap_or_default();
                let weight = weight.extend(dep_target_kind);
                graph.add_edge(package_id, dep, weight);
                summaries.insert(dep, dep_summary.clone());

                if packages.contains_key(dep.name.as_ref()) {
                    continue;
                }

                packages.insert(dep.name.clone(), dep);
                next_queue.push(dep);
            }
        }

        queue = next_queue;
    }

    // Detect incompatibilities and bail in case ones are found.
    let mut incompatibilities = Vec::new();
    for from_package in graph.nodes() {
        for manifest_dependency in summaries[&from_package].full_dependencies() {
            let to_package = packages[&manifest_dependency.name];
            if !manifest_dependency.matches_package_id(to_package) {
                let message = format!(
                    "- {from_package} cannot use {to_package}, because {} requires {} {}",
                    from_package.name, to_package.name, manifest_dependency.version_req
                );
                incompatibilities.push(message);
            }
        }
    }

    if !incompatibilities.is_empty() {
        incompatibilities.sort();
        let incompatibilities = incompatibilities.join("\n");
        bail!(formatdoc! {"
            Version solving failed:
            {incompatibilities}

            Scarb does not have real version solving algorithm yet.
            Perhaps in the future this conflict could be resolved, but currently,
            please upgrade your dependencies to use latest versions of their dependencies.
        "});
    }

    Ok(Resolve { graph })
}

async fn rewrite_dependency_source_id(
    registry: &dyn Registry,
    package_id: &PackageId,
    dependency: &ManifestDependency,
) -> Result<ManifestDependency> {
    // Rewrite path dependencies for git sources.
    if package_id.source_id.is_git() && dependency.source_id.is_path() {
        let rewritten_dep = ManifestDependency::builder()
            .kind(dependency.kind.clone())
            .name(dependency.name.clone())
            .source_id(package_id.source_id)
            .version_req(dependency.version_req.clone())
            .build();
        // Check if this dependency can be queried from git source.
        // E.g. packages below other package's manifest will not be accessible.
        if !registry.query(&rewritten_dep).await?.is_empty() {
            // If it is, return rewritten dependency.
            return Ok(rewritten_dep);
        }
    };

    Ok(dependency.clone())
}

#[cfg(test)]
mod tests {
    //! These tests largely come from Elixir's `hex_solver` test suite.

    // TODO(mkaput): Remove explicit path source IDs, when we will support default registry.

    use indoc::indoc;
    use itertools::Itertools;
    use semver::Version;
    use similar_asserts::assert_serde_eq;
    use tokio::runtime::Builder;

    use crate::core::package::PackageName;
    use crate::core::registry::mock::{deps, pkgs, registry, MockRegistry};
    use crate::core::{ManifestDependency, PackageId, Resolve, SourceId};

    fn check(
        registry: MockRegistry,
        roots: &[&[ManifestDependency]],
        expected: Result<&[PackageId], &str>,
    ) {
        let root_ids = (1..).map(|n| package_id(format!("root_{n}")));

        let roots = roots
            .iter()
            .zip(root_ids)
            .map(|(&deps, pid)| (deps, pid))
            .collect_vec();

        let resolve = resolve(registry, roots);

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
        mut registry: MockRegistry,
        roots: Vec<(&[ManifestDependency], PackageId)>,
    ) -> anyhow::Result<Resolve> {
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

        runtime.block_on(super::resolve(&summaries, &registry))
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
            //     "baz v1.8.0",
            //     "foo v2.9.0"
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

        let mut test_solution = resolve.solution_with_target_kind(root, "test".into());
        test_solution.sort();
        assert_eq!(test_solution.len(), 5);
        assert_eq!(
            test_solution
                .iter()
                .map(|p| p.name.clone().to_string())
                .collect_vec(),
            vec!["bar", "boo", "core", "foo", "testplugin"],
        );

        let mut lib_solution = resolve.solution_with_target_kind(root, "lib".into());
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
    #[ignore = "locks are not implemented yet"]
    fn lock_dependency() {
        //     Registry.put("foo", "1.0.0", [])
        //
        //     assert run([{"foo", "1.0.0"}], [{"foo", "1.0.0"}]) == %{"foo" => "1.0.0"}
    }

    #[test]
    #[ignore = "locks are not implemented yet"]
    fn lock_conflict_1() {
        //     Registry.put("foo", "1.0.0", [])
        //
        //     assert {:conflict, incompatibility, _} = run([{"foo", "1.0.0"}], [{"foo", "2.0.0"}])
        //     assert [term] = incompatibility.terms
        //     assert term.package_range.name == "foo"
        //     assert term.package_range.constraint == Version.parse!("2.0.0")
        //     assert {:conflict, _, _} = incompatibility.cause
    }

    #[test]
    #[ignore = "locks are not implemented yet"]
    fn lock_conflict_2() {
        //     Registry.put("foo", "1.0.0", [])
        //
        //     assert {:conflict, incompatibility, _} = run([{"foo", "2.0.0"}], [{"foo", "1.0.0"}])
        //     assert [term] = incompatibility.terms
        //     assert term.package_range.name == "foo"
        //     assert term.package_range.constraint == Version.parse!("2.0.0")
        //     assert incompatibility.cause == :no_versions
    }

    #[test]
    #[ignore = "locks are not implemented yet"]
    fn lock_downgrade() {
        //     Registry.put("foo", "1.0.0", [])
        //     Registry.put("foo", "1.1.0", [])
        //     Registry.put("foo", "1.2.0", [])
        //
        //     assert run([{"foo", "~1.0"}], [{"foo", "1.1.0"}]) == %{"foo" => "1.1.0"}
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
