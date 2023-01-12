use std::fmt;

use crate::core::SourceId;
use crate::resolver::package_ref::PackageRef;
use crate::resolver::version_constraint::VersionConstraint;

#[derive(Clone, Debug)]
pub struct PackageRange {
    pub name: PackageRef,
    pub constraint: VersionConstraint,
    pub source_id: Option<SourceId>,
}

impl PackageRange {
    pub fn without_source(name: PackageRef, constraint: VersionConstraint) -> Self {
        Self {
            name,
            constraint,
            source_id: None,
        }
    }
}

impl fmt::Display for PackageRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;

        if !self.constraint.is_all() {
            write!(f, "{}", self.constraint)?;
        }

        if let Some(source_id) = self.source_id {
            write!(f, " ({})", source_id)?;
        }

        Ok(())
    }
}
