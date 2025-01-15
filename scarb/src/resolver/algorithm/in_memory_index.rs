use crate::core::{ManifestDependency, Summary};
use once_map::OnceMap;
use std::sync::Arc;

/// In-memory index of package metadata.
#[derive(Default, Clone)]
pub struct InMemoryIndex(Arc<SharedInMemoryIndex>);

#[derive(Default)]
struct SharedInMemoryIndex {
    /// A map from package name to the metadata for that package and the index where the metadata
    /// came from.
    packages: OnceMap<ManifestDependency, Arc<VersionsResponse>>,
}

impl InMemoryIndex {
    /// Returns a reference to the package metadata.
    pub fn packages(&self) -> &OnceMap<ManifestDependency, Arc<VersionsResponse>> {
        &self.0.packages
    }
}

#[derive(Debug)]
pub enum VersionsResponse {
    Found(Vec<Summary>),
}
