use std::collections::BTreeMap;
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

use semver::VersionReq;
use serde::{Deserialize, Serialize};

pub use toml::*;

use crate::core::package::{PackageId, PackageName};
use crate::core::source::SourceId;

mod toml;

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
    pub no_core: bool,
}

impl Deref for Summary {
    type Target = SummaryInner;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl Summary {
    pub fn build(package_id: PackageId) -> SummaryBuilder {
        SummaryBuilder::new(package_id)
    }

    pub fn minimal(package_id: PackageId, dependencies: Vec<ManifestDependency>) -> Self {
        Self::build(package_id)
            .with_dependencies(dependencies)
            .finish()
    }

    fn new(data: SummaryInner) -> Self {
        Self(Arc::new(data))
    }
}

#[derive(Debug)]
pub struct SummaryBuilder {
    package_id: PackageId,
    dependencies: Vec<ManifestDependency>,
    no_core: bool,
}

impl SummaryBuilder {
    fn new(package_id: PackageId) -> Self {
        Self {
            package_id,
            dependencies: Vec::new(),
            no_core: false,
        }
    }

    pub fn with_dependencies(mut self, dependencies: Vec<ManifestDependency>) -> Self {
        self.dependencies = dependencies;
        self
    }

    pub fn no_core(mut self, no_core: bool) -> Self {
        self.no_core = no_core;
        self
    }

    pub fn finish(self) -> Summary {
        Summary::new(SummaryInner {
            package_id: self.package_id,
            dependencies: self.dependencies,
            no_core: self.no_core,
        })
    }
}

/// Subset of a [`Manifest`] that contains package metadata.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ManifestMetadata {
    pub authors: Option<Vec<String>>,
    pub urls: Option<BTreeMap<String, String>>,
    #[serde(rename = "metadata")]
    pub custom_metadata: Option<BTreeMap<String, String>>,
    pub description: Option<String>,
    pub documentation: Option<String>,
    pub homepage: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub license: Option<String>,
    pub license_file: Option<String>,
    pub readme: Option<String>,
    pub repository: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ManifestDependency {
    pub name: PackageName,
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

impl fmt::Display for ManifestDependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.name, self.version_req)?;

        if !self.source_id.is_default_registry() {
            write!(f, " ({})", self.source_id)?;
        }

        Ok(())
    }
}
