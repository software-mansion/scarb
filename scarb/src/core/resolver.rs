use itertools::Itertools;
use petgraph::graphmap::DiGraphMap;
use petgraph::visit::{Dfs, EdgeFiltered, Walker};
use smallvec::SmallVec;

use crate::core::{PackageId, TargetKind};

// TODO(#126): Produce lockfile out of this.
/// Represents a fully-resolved package dependency graph.
///
/// Each node in the graph is a package and edges represent dependencies between packages.
#[derive(Debug)]
pub struct Resolve {
    /// Directional graph representing package dependencies.
    ///
    /// If package `a` depends on package `b`, then this graph will contain an edge from `a` to `b`.
    pub graph: DiGraphMap<PackageId, DependencyEdge>,
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
        let filtered_graph = EdgeFiltered::from_fn(&self.graph, move |(_node_a, _node_b, edge)| {
            edge.accepts_target(target_kind.clone())
        });
        Dfs::new(&filtered_graph, root_package)
            .iter(&filtered_graph)
            .unique()
            .collect_vec()
    }

    /// Collect `[PackageId`]s of all directed dependencies of the package.
    pub fn package_dependencies(
        &self,
        package_id: PackageId,
    ) -> impl Iterator<Item = PackageId> + '_ {
        self.graph
            .neighbors_directed(package_id, petgraph::Direction::Outgoing)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DependencyEdge(SmallVec<[TargetKind; 1]>);

impl DependencyEdge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn accepts_target(&self, target_kind: TargetKind) -> bool {
        // Empty target lists accepts all target kinds.
        // Represents `[dependencies]` table from manifest file.
        self.0.is_empty() || self.0.iter().any(|name| target_kind == *name)
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
