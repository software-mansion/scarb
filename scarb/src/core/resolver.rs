use std::collections::HashMap;

use anyhow::{bail, Result};
use indoc::formatdoc;
use itertools::Itertools;
use petgraph::graphmap::DiGraphMap;
use petgraph::visit::{Dfs, EdgeFiltered, IntoNeighborsDirected, Walker};
use smallvec::SmallVec;

use crate::core::lockfile::Lockfile;
use crate::core::{PackageId, Summary, TargetKind};

/// Represents a fully-resolved package dependency graph.
///
/// Each node in the graph is a package and edges represent dependencies between packages.
#[derive(Debug)]
pub struct Resolve {
    /// Directional graph representing package dependencies.
    ///
    /// If package `a` depends on package `b`, then this graph will contain an edge from `a` to `b`.
    pub graph: DiGraphMap<PackageId, DependencyEdge>,
    /// Summaries of all packages in the graph.
    pub summaries: HashMap<PackageId, Summary>,
}

impl Resolve {
    /// Iterator over all [`PackageId`]s (nodes) present in this graph.
    ///
    /// This is an easier to discover shortcut for `self.graph.nodes()`.
    pub fn package_ids(&self) -> impl Iterator<Item = PackageId> + '_ {
        self.graph.nodes()
    }

    /// Collect all [`PackageId`]s needed to compile a root package.
    ///
    /// Returns a collection of all [`PackageId`]s of packages needed to provide as _crate roots_
    /// to the Cairo compiler, or to load as _cairo plugins_, in order to build a particular
    /// package (named _root package_).
    ///
    /// # Safety
    /// * Asserts that `root_package` is a node in this graph.
    pub fn solution_of(&self, root_package: PackageId, target_kind: &TargetKind) -> Vec<PackageId> {
        assert!(&self.graph.contains_node(root_package));
        let filtered_graph = EdgeFiltered::from_fn(&self.graph, move |(node_a, _node_b, edge)| {
            edge.accepts_target(target_kind.clone(), node_a == root_package)
        });
        Dfs::new(&filtered_graph, root_package)
            .iter(&filtered_graph)
            .unique()
            .collect_vec()
    }

    /// Collect [`PackageId`]s of all directed dependencies of the package.
    pub fn package_dependencies(
        &self,
        package_id: PackageId,
    ) -> impl Iterator<Item = PackageId> + '_ {
        self.graph
            .neighbors_directed(package_id, petgraph::Direction::Outgoing)
    }

    /// Collect [`PackageId`]s of directed dependencies of the package, that accept the given target kind.
    pub fn package_dependencies_for_target_kind(
        &self,
        package_id: PackageId,
        target_kind: &TargetKind,
    ) -> Vec<PackageId> {
        let filtered_graph = EdgeFiltered::from_fn(&self.graph, move |(node_a, _node_b, edge)| {
            edge.accepts_target(target_kind.clone(), node_a == package_id)
        });
        filtered_graph
            .neighbors_directed(package_id, petgraph::Direction::Outgoing)
            .collect_vec()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DependencyEdge(SmallVec<[TargetKind; 1]>);

impl DependencyEdge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn accepts_target(&self, target_kind: TargetKind, is_root: bool) -> bool {
        if self.0.is_empty() {
            // Empty target lists accepts all target kinds.
            // Represents `[dependencies]` table from manifest file.
            return true;
        }
        // For `TargetKind::TEST`, we should not consider the root package dependencies.
        (is_root || target_kind != TargetKind::TEST)
            && self.0.iter().any(|name| target_kind == *name)
    }

    pub fn extend(self, target_kind: Option<TargetKind>) -> Self {
        if let Some(target_kind) = target_kind {
            let mut edge = self.0;
            edge.push(target_kind);
            Self(edge)
        } else {
            // For None, create empty vector to accept all targets.
            Self::default()
        }
    }
}

impl From<Vec<TargetKind>> for DependencyEdge {
    fn from(target_kinds: Vec<TargetKind>) -> Self {
        Self(target_kinds.into())
    }
}

impl From<TargetKind> for DependencyEdge {
    fn from(target_kind: TargetKind) -> Self {
        Self(vec![target_kind].into())
    }
}

/// Lockfiles handling.
impl Resolve {
    /// Check that the newly generated resolve is compliant with the previous one generated
    /// from a lock file.
    ///
    /// Given an existing lock file, it should be forbidden to ever have a checksums which
    /// *differ*. If the same package ids' summaries have differing checksums, then something
    /// has gone wrong such as:
    ///
    /// * something got seriously corrupted,
    /// * a "mirror" is not actually a mirror as some changes were made,
    /// * a replacement source was not actually a replacement, some changes were made.
    ///
    /// In all of these cases, we want to report an error to indicate that something is awry.
    /// Normal execution (esp. just using the default registry) should never run into this.
    pub fn check_checksums(&self, lockfile: &Lockfile) -> Result<()> {
        for package_lock in &lockfile.packages {
            let (locked, source_id) = match (package_lock.checksum.as_ref(), package_lock.source) {
                (None, None) => continue,
                (Some(_), None) => {
                    unreachable!(
                        "Package lock entry `{n} v{v}` has `checksum` but no `source` field.",
                        n = package_lock.name,
                        v = package_lock.version
                    );
                }
                (locked, Some(source_id)) => (locked, source_id),
            };

            let id = PackageId::new(
                package_lock.name.clone(),
                package_lock.version.clone(),
                source_id,
            );

            let Some(actual) = self.summaries.get(&id).map(|s| s.checksum.as_ref()) else {
                continue;
            };

            match (actual, locked) {
                // If the checksums are the same, or both are not present, then we are good.
                (Some(actual), Some(locked)) if actual == locked => {}
                (None, None) => {}

                // If the locked checksum was not calculated, and the current checksum is `Some`,
                // it may indicate that a source was erroneously replaced or was replaced with
                // something that desires stronger checksum guarantees than can be afforded
                // elsewhere.
                (Some(_), None) => {
                    bail!(formatdoc! {"
                        checksum for `{id}` was not previously calculated, but now it could be

                        this could be indicative of a few possible situations:

                            * the source `{source_id}` did not previously support checksums, \
                              but was replaced with one that does
                            * newer Scarb implementations know how to checksum this source, \
                              but this older implementation does not
                            * the lock file is corrupt
                    "});
                }

                // If our checksum has not been calculated, then it could mean that future Scarb
                // figured out how to do it or the source has been shadowed by with
                // a different one thanks to some unknown future logic.
                (None, Some(_)) => {
                    bail!(formatdoc! {"
                        checksum for `{id}` could not be calculated, but a checksum is listed in \
                        the existing lock file

                        this could be indicative of a few possible situations:

                            * the source `{source_id}` supports checksums, \
                              but was replaced with one that does not
                            * the lock file is corrupt

                        unable to verify that `{id}` is the same as when the lockfile was generated
                    "});
                }

                // Both checksums are known, but they differ.
                (Some(_), Some(_)) => {
                    bail!(formatdoc! {"
                        checksum for `{id}` changed between lock files

                        this could be indicative of a few possible errors:

                            * the lock file is corrupt
                            * a replacement source in use (e.g. a mirror) returned a different \
                              checksum
                            * the source itself may be corrupt in one way or another

                        unable to verify that `{id}` is the same as when the lockfile was generated
                    "});
                }
            }
        }
        Ok(())
    }
}
