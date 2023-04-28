use anyhow::Result;
use async_trait::async_trait;
use futures::prelude::*;

use crate::core::registry::Registry;
use crate::core::{ManifestDependency, Package, PackageId, Summary};
use crate::internal::async_cache::AsyncCache;

/// A caching wrapper over another [`Registry`] which memorizes all queries and downloads.
pub struct RegistryCache<'a> {
    queries: AsyncCache<'a, ManifestDependency, Vec<Summary>, &'a dyn Registry>,
    downloads: AsyncCache<'a, PackageId, Package, &'a dyn Registry>,
}

impl<'a> RegistryCache<'a> {
    pub fn new(registry: &'a dyn Registry) -> Self {
        Self {
            queries: AsyncCache::new(registry, {
                move |dependency, registry| {
                    async move { Ok(registry.query(&dependency).await?) }.boxed_local()
                }
            }),
            downloads: AsyncCache::new(registry, {
                move |package_id, registry| {
                    async move { Ok(registry.download(package_id).await?) }.boxed_local()
                }
            }),
        }
    }
}

#[async_trait(?Send)]
impl<'c> Registry for RegistryCache<'c> {
    /// Attempt to find the packages that match dependency request.
    async fn query(&self, dependency: &ManifestDependency) -> Result<Vec<Summary>> {
        self.queries.load(dependency.clone()).await
    }

    /// Fetch full package by its ID.
    async fn download(&self, package_id: PackageId) -> Result<Package> {
        self.downloads.load(package_id).await
    }
}
