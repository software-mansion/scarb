use std::fmt;

use crate::core::package::PackageName;

/// Superset of [`PackageName`] which includes special meta-packages used to drive solver.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PackageRef {
    Root,
    Package(PackageName),
    Lock,
}

impl fmt::Display for PackageRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageRef::Root => write!(f, "your package"),
            PackageRef::Package(name) => write!(f, "{name}"),
            PackageRef::Lock => write!(f, "the lock"),
        }
    }
}
