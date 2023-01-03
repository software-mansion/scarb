use semver::VersionReq;
use smol_str::SmolStr;

use crate::core::package::PackageId;
use crate::core::source::SourceId;
pub use toml::*;

mod toml;

pub const MANIFEST_FILE_NAME: &str = "Murek.toml";

/// Contains all the information about a package, as loaded from the manifest file.
///
/// This is deserialized using the [`TomlManifest`] type.
#[derive(Clone, Debug)]
pub struct Manifest {
    pub summary: Summary,
}

/// Subset of a [`Manifest`] that contains only the most important information about a package.
#[derive(Clone, Debug)]
pub struct Summary {
    pub package_id: PackageId,
    pub dependencies: Vec<ManifestDependency>,
}

#[derive(Clone, Debug)]
pub struct ManifestDependency {
    pub name: SmolStr,
    pub version_req: VersionReq,
    pub source_id: SourceId,
}

impl ManifestDependency {
    pub fn matches_summary(&self, summary: &Summary) -> bool {
        self.matches_package_id(summary.package_id)
    }

    pub fn matches_package_id(&self, package_id: PackageId) -> bool {
        package_id.name == self.name && self.version_req.matches(&package_id.version)
    }
}
