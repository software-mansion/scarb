use crate::core::{
    DependencyVersionReq, FeatureName, PackageId, PackageName, SourceId, Summary, TargetKind,
};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;
use typed_builder::TypedBuilder;

/// See [`ManifestDependencyInner`] for public fields reference.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ManifestDependency(Arc<ManifestDependencyInner>);

#[derive(TypedBuilder, Clone, Eq, PartialEq, Hash)]
#[builder(builder_type(name = ManifestDependencyBuilder))]
#[builder(builder_method(vis = ""))]
#[builder(build_method(into = ManifestDependency))]
pub struct ManifestDependencyInner {
    pub name: PackageName,
    pub version_req: DependencyVersionReq,
    #[builder(default)]
    pub source_id: SourceId,
    #[builder(default)]
    pub kind: DepKind,
    #[builder(default)]
    pub features: Vec<FeatureName>,
    #[builder(default = true)]
    pub default_features: bool,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(try_from = "serdex::DepKind", into = "serdex::DepKind")]
pub enum DepKind {
    #[default]
    Normal,
    Target(TargetKind),
}

impl DepKind {
    pub fn is_propagated(&self) -> bool {
        !self.is_test()
    }

    pub fn is_test(&self) -> bool {
        match self {
            DepKind::Target(kind) => kind.is_test(),
            _ => false,
        }
    }
}

mod serdex {
    use crate::core::TargetKind;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize)]
    pub struct DepKind(String);

    impl TryFrom<DepKind> for super::DepKind {
        type Error = serde::de::value::Error;

        fn try_from(value: DepKind) -> Result<Self, Self::Error> {
            if value.0 == "normal" {
                Ok(super::DepKind::Normal)
            } else {
                Ok(super::DepKind::Target(
                    TargetKind::try_new(value.0.as_str())
                        .map_err(|e| serde::de::Error::custom(e.to_string()))?,
                ))
            }
        }
    }

    impl From<super::DepKind> for DepKind {
        fn from(value: super::DepKind) -> Self {
            match value {
                super::DepKind::Target(t) => DepKind(t.to_string()),
                super::DepKind::Normal => DepKind("normal".to_string()),
            }
        }
    }
}

impl Deref for ManifestDependency {
    type Target = ManifestDependencyInner;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

#[doc(hidden)]
impl From<ManifestDependencyInner> for ManifestDependency {
    fn from(data: ManifestDependencyInner) -> Self {
        Self(Arc::new(data))
    }
}

impl ManifestDependency {
    pub fn builder() -> ManifestDependencyBuilder {
        ManifestDependencyInner::builder()
    }

    pub fn exact_for_package(package_id: PackageId) -> Self {
        Self::builder()
            .name(package_id.name.clone())
            .version_req(DependencyVersionReq::exact(&package_id.version))
            .source_id(package_id.source_id)
            .build()
    }

    pub fn matches_summary(&self, summary: &Summary) -> bool {
        self.matches_package_id(summary.package_id)
    }

    pub fn matches_package_id(&self, package_id: PackageId) -> bool {
        self.matches_name_and_version(&package_id.name, &package_id.version)
    }

    pub fn matches_name_and_version(&self, name: &PackageName, version: &Version) -> bool {
        *name == self.name && self.version_req.matches(version)
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
