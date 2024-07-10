use crate::core::{Package, PackageId, Summary};
use crate::resolver::algorithm::provider::PubGrubPackage;
use once_map::OnceMap;
use std::sync::Arc;

/// In-memory index of package metadata.
#[derive(Default, Clone)]
pub struct InMemoryIndex(Arc<SharedInMemoryIndex>);

#[derive(Default)]
struct SharedInMemoryIndex {
    /// A map from package name to the metadata for that package and the index where the metadata
    /// came from.
    packages: FxOnceMap<PubGrubPackage, Arc<VersionsResponse>>,

    /// A map from package ID to metadata for that distribution.
    distributions: FxOnceMap<PackageId, Arc<MetadataResponse>>,
}

pub(crate) type FxOnceMap<K, V> = OnceMap<K, V>;

impl InMemoryIndex {
    /// Returns a reference to the package metadata map.
    pub fn packages(&self) -> &FxOnceMap<PubGrubPackage, Arc<VersionsResponse>> {
        &self.0.packages
    }

    /// Returns a reference to the distribution metadata map.
    pub fn distributions(&self) -> &FxOnceMap<PackageId, Arc<MetadataResponse>> {
        &self.0.distributions
    }
}

// pub struct VersionsResponse;
#[derive(Debug)]
pub enum VersionsResponse {
    Found(Vec<Summary>),
}

// pub struct MetadataResponse;
pub enum MetadataResponse {
    Found(Package),
}
