use anyhow::Result;
use async_trait::async_trait;

use crate::core::registry::source_map::SourceMap;
use crate::core::registry::Registry;
use crate::core::{ManifestDependency, Package, PackageId, Summary};

// TODO(mkaput): Really implement what is promised here.
/// A caching wrapper over another [`Registry`] which memorizes all queries and downloads.
pub struct RegistryCache<'c> {
    registry: SourceMap<'c>,
}

impl<'c> RegistryCache<'c> {
    pub fn new(registry: SourceMap<'c>) -> Self {
        Self { registry }
    }
}

#[async_trait(?Send)]
impl<'c> Registry for RegistryCache<'c> {
    /// Attempt to find the packages that match dependency request.
    async fn query(&self, dependency: &ManifestDependency) -> Result<Vec<Summary>> {
        self.registry.query(dependency).await
    }

    /// Fetch full package by its ID.
    async fn download(&self, package_id: PackageId) -> Result<Package> {
        self.registry.download(package_id).await
    }
}
