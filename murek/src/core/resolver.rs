use std::collections::{HashMap, HashSet};

use crate::core::PackageId;

// TODO(mkaput): Produce lockfile out of this.
/// Represents a fully-resolved package dependency graph.
///
/// Each node in the graph is a package and edges represent dependencies between packages.
pub struct Resolve {
    pub package_ids: HashSet<PackageId>,
    pub compilation_units: HashMap<PackageId, HashSet<PackageId>>,
}
