use anyhow::Result;

use crate::core::registry::Registry;
use crate::core::{ManifestDependency, Package, PackageId, Summary};

// TODO(mkaput): Really implement what is promised here.
/// A caching wrapper over [`Registry`] which memorizes all queries and downloads.
pub struct RegistryCache<'c> {
    registry: Registry<'c>,
}

impl<'c> RegistryCache<'c> {
    pub fn new(registry: Registry<'c>) -> Self {
        Self { registry }
    }

    /// Attempt to find the packages that match dependency request.
    pub async fn query(&mut self, dependency: &ManifestDependency) -> Result<Vec<Summary>> {
        self.registry.query(dependency).await
    }

    /// Fetch full package by its ID.
    pub async fn download(&mut self, package_id: PackageId) -> Result<Package> {
        self.registry.download(package_id).await
    }
}
