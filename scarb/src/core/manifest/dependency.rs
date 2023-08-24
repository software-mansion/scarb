use std::fmt;

use crate::core::{DependencyVersionReq, PackageId, PackageName, SourceId, Summary, TargetKind};

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ManifestDependency {
    pub name: PackageName,
    pub version_req: DependencyVersionReq,
    pub source_id: SourceId,
    pub target_kind: Option<TargetKind>,
}

impl ManifestDependency {
    pub fn for_all_targets(
        name: PackageName,
        version_req: DependencyVersionReq,
        source_id: SourceId,
    ) -> Self {
        Self {
            name,
            version_req,
            source_id,
            target_kind: None,
        }
    }

    pub fn for_target_kind(
        name: PackageName,
        version_req: DependencyVersionReq,
        source_id: SourceId,
        target_kind: TargetKind,
    ) -> Self {
        Self {
            name,
            version_req,
            source_id,
            target_kind: Some(target_kind),
        }
    }

    pub fn matches_summary(&self, summary: &Summary) -> bool {
        self.matches_package_id(summary.package_id)
    }

    pub fn matches_package_id(&self, package_id: PackageId) -> bool {
        package_id.name == self.name && self.version_req.matches(&package_id.version)
    }
}

impl fmt::Display for ManifestDependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.name, self.version_req)?;

        if !self.source_id.is_default_registry() {
            write!(f, " ({})", self.source_id)?;
        }

        Ok(())
    }
}

impl fmt::Debug for ManifestDependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ManifestDependency({self})")
    }
}
