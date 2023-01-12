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

/// Collection of all [`PackageId`]s or packages needed to provide as _crate roots_ to
/// the Cairo compiler in order to build a particular package (named _root package_).
pub type CompilationUnit = HashSet<PackageId>;

impl Resolve {
    /// Iterator over all [`PackageId`]s (nodes) present in this graph.
    ///
    /// This is an easier to discover shortcut for `self.graph.nodes()`.
    pub fn package_ids(&self) -> impl Iterator<Item = PackageId> + '_ {
        self.graph.nodes()
    }

    /// Construct [`CompilationUnit`] for a root package.
    pub fn collect_compilation_unit_of(&self, root_package: PackageId) -> CompilationUnit {
        assert!(&self.graph.contains_node(root_package));
        Dfs::new(&self.graph, root_package)
            .iter(&self.graph)
            .collect()
    }
}
