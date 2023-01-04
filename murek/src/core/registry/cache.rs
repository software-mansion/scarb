use anyhow::Result;

use crate::core::registry::Registry;
use crate::core::{ManifestDependency, Package, PackageId, Summary};

// TODO(mkaput): Really implement what is promised here.
/// A caching wrapper over [`Registry`] which memorizes all queries and downloads results
/// throughout its lifetime and manages parallel async execution under the hood.
pub struct RegistryCache<'c> {
    registry: Registry<'c>,
}

impl<'c> RegistryCache<'c> {
    pub fn new(registry: Registry<'c>) -> Self {
        Self { registry }
    }

    /// Attempt to find the packages that match dependency requests in a batch.
    pub fn query_many(
        &mut self,
        dependencies: &[&ManifestDependency],
    ) -> Result<Vec<Vec<Summary>>> {
        smol::block_on(async {
            let mut results = Vec::with_capacity(dependencies.len());
            for dependency in dependencies {
                let result = self.registry.query(dependency).await?;
                results.push(result);
            }
            Ok(results)
        })
    }

    /// Fetch full package by its ID.
    pub fn download(&mut self, package: PackageId) -> Result<Package> {
        self.download_many(&[package])
            .map(|v| v.into_iter().next().unwrap())
    }

    /// Fetch full packages by their IDs in a batch.
    pub fn download_many(&mut self, packages: &[PackageId]) -> Result<Vec<Package>> {
        smol::block_on(self.registry.download_many(packages))
    }
}
