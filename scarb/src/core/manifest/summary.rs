use crate::core::{ManifestDependency, PackageId, PackageName, SourceId};
use once_cell::sync::Lazy;
use semver::VersionReq;
use std::ops::Deref;
use std::sync::Arc;

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
                source_id: SourceId::default(),
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
