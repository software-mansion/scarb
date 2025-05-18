use crate::core::{ManifestDependency, Summary};
use once_map::OnceMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// In-memory index of package metadata.
#[derive(Default, Clone)]
pub struct InMemoryIndex(Arc<SharedInMemoryIndex>);

#[derive(Default)]
struct SharedInMemoryIndex {
    /// A map from package name to the metadata for that package and the index where the metadata
    /// came from.
    packages: OnceMap<ManifestDependencySourceKey, Arc<VersionsResponse>>,
}

impl InMemoryIndex {
    /// Returns a reference to the package metadata.
    pub fn packages(&self) -> &OnceMap<ManifestDependencySourceKey, Arc<VersionsResponse>> {
        &self.0.packages
    }
}

/// This struct defines keys that will be used to deduplicate requests to access dependency metadata.
/// This ignores all fields in `ManifestDependency` that are internal to Scarb (e.g. enabled features),
/// and only considers fields that differentiate package or package source.
///
/// This can be easily converted to and from `ManifestDependency` using the `From` trait.
#[derive(Clone)]
pub struct ManifestDependencySourceKey(ManifestDependency);

impl Hash for ManifestDependencySourceKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.name.hash(state);
        self.0.source_id.hash(state);
        self.0.version_req.hash(state);
    }
}

impl PartialEq for ManifestDependencySourceKey {
    fn eq(&self, other: &Self) -> bool {
        self.0.name == other.0.name
            && self.0.source_id == other.0.source_id
            && self.0.version_req == other.0.version_req
    }
}

impl Eq for ManifestDependencySourceKey {}

impl From<ManifestDependency> for ManifestDependencySourceKey {
    fn from(dependency: ManifestDependency) -> Self {
        Self(dependency)
    }
}

impl From<ManifestDependencySourceKey> for ManifestDependency {
    fn from(dependency: ManifestDependencySourceKey) -> Self {
        dependency.0
    }
}

#[derive(Debug)]
pub enum VersionsResponse {
    Found(Vec<Summary>),
}
