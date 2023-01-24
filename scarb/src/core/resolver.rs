use std::collections::HashSet;

use petgraph::graphmap::DiGraphMap;
use petgraph::visit::{Dfs, Walker};

use crate::core::PackageId;

// TODO(mkaput): Produce lockfile out of this.
/// Represents a fully-resolved package dependency graph.
///
/// Each node in the graph is a package and edges represent dependencies between packages.
#[derive(Debug)]
pub struct Resolve {
    /// Directional graph representing package dependencies.
    ///
    /// If package `a` depends on package `b`, then this graph will contain an edge from `a` to `b`.
    pub graph: DiGraphMap<PackageId, ()>,
}

/// Collection of all [`PackageId`]s of packages needed to provide as _crate roots_ to
/// the Cairo compiler in order to build a particular package (named _root package_).
pub type PackageComponentsIds = HashSet<PackageId>;

impl Resolve {
    /// Iterator over all [`PackageId`]s (nodes) present in this graph.
    ///
    /// This is an easier to discover shortcut for `self.graph.nodes()`.
    pub fn package_ids(&self) -> impl Iterator<Item = PackageId> + '_ {
        self.graph.nodes()
    }

    /// Collect all [`PackageId`]s needed to compile a root package.
    ///
    /// # Safety
    /// * Asserts that `root_package` is a node in this graph.
    pub fn package_components_of(&self, root_package: PackageId) -> PackageComponentsIds {
        assert!(&self.graph.contains_node(root_package));
        Dfs::new(&self.graph, root_package)
            .iter(&self.graph)
            .collect()
    }
}
