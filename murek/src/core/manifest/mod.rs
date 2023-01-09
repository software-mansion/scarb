use semver::VersionReq;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::collections::BTreeMap;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

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
    pub metadata: ManifestMetadata,
}

/// Subset of a [`Manifest`] that contains only the most important information about a package.
/// See [`SummaryInner`] for public fields reference.
#[derive(Clone, Debug)]
pub struct Summary(Arc<SummaryInner>);

#[derive(Debug)]
#[non_exhaustive]
pub struct SummaryInner {
    pub package_id: PackageId,
    pub dependencies: Vec<ManifestDependency>,
}

impl Deref for Summary {
    type Target = SummaryInner;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl Summary {
    pub fn new(package_id: PackageId, dependencies: Vec<ManifestDependency>) -> Self {
        Self(Arc::new(SummaryInner {
            package_id,
            dependencies,
        }))
    }
}

/// Subset of a [`Manifest`] that contains package metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManifestMetadata {
    pub authors: Option<Vec<String>>,
    pub custom_links: Option<BTreeMap<String, String>>,
    pub custom_metadata: Option<BTreeMap<String, String>>,
    pub description: Option<String>,
    pub documentation: Option<String>,
    pub homepage: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub license: Option<String>,
    pub license_file: Option<PathBuf>,
    pub readme: Option<PathBuf>,
    pub repository: Option<String>,
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
