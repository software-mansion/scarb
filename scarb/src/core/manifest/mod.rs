use std::collections::BTreeMap;
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

use once_cell::sync::Lazy;
use semver::VersionReq;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use toml::Value;

pub use scripts::*;
pub use target::*;
pub use toml_manifest::*;

use crate::core::package::{PackageId, PackageName};
use crate::core::source::SourceId;

mod scripts;
mod target;
mod toml_manifest;

/// Contains all the information about a package, as loaded from the manifest file.
///
/// This is deserialized using the [`TomlManifest`] type.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Manifest {
    pub summary: Summary,
    pub targets: Vec<Target>,
    pub metadata: ManifestMetadata,
    pub compiler_config: ManifestCompilerConfig,
    pub scripts: BTreeMap<SmolStr, ScriptDefinition>,
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

    pub fn full_dependencies(&self) -> impl Iterator<Item = &ManifestDependency> {
        self.dependencies.iter().chain(self.implicit_dependencies())
    }

    pub fn implicit_dependencies(&self) -> impl Iterator<Item = &ManifestDependency> {
        static CORE_DEPENDENCY: Lazy<ManifestDependency> = Lazy::new(|| {
            // NOTE: Pin `core` to exact version, because we know that's the only one we have.
            let cairo_version = crate::version::get().cairo.version;
            let version_req = VersionReq::parse(&format!("={cairo_version}")).unwrap();
            ManifestDependency {
                name: PackageName::CORE,
                version_req,
                source_id: SourceId::for_std(),
            }
        });

        let mut deps: Vec<&ManifestDependency> = Vec::new();

        if !self.no_core {
            deps.push(&CORE_DEPENDENCY);
        }

        deps.into_iter()
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
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ManifestMetadata {
    pub authors: Option<Vec<String>>,
    pub urls: Option<BTreeMap<String, String>>,
    pub description: Option<String>,
    pub documentation: Option<String>,
    pub homepage: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub license: Option<String>,
    pub license_file: Option<String>,
    pub readme: Option<String>,
    pub repository: Option<String>,
    #[serde(rename = "tool")]
    pub tool_metadata: Option<BTreeMap<SmolStr, Value>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, Hash)]
pub struct ManifestCompilerConfig {
    /// Replace all names in generated Sierra code with dummy counterparts, representing the
    /// expanded information about the named items.
    ///
    /// For libfuncs and types that would be recursively opening their generic arguments.
    /// For functions, that would be their original name in Cairo.
    /// For example, while the Sierra name be `[6]`, with this flag turned on it might be:
    /// - For libfuncs: `felt252_const<2>` or `unbox<Box<Box<felt252>>>`.
    /// - For types: `felt252` or `Box<Box<felt252>>`.
    /// - For user functions: `test::foo`.
    pub sierra_replace_ids: bool,
}

#[derive(Clone, Eq, PartialEq, Hash)]
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

impl fmt::Debug for ManifestDependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ManifestDependency({self})")
    }
}
