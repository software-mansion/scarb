use std::path::PathBuf;

use anyhow::{bail, Result};
use tracing::trace;

use crate::core::registry::client::{RegistryClient, RegistryResource};
use crate::core::registry::index::IndexRecords;
use crate::core::{Config, ManifestDependency, PackageId};

pub struct RegistryClientCache<'c> {
    client: Box<dyn RegistryClient + 'c>,
    _config: &'c Config,
}

impl<'c> RegistryClientCache<'c> {
    pub fn new(client: Box<dyn RegistryClient + 'c>, config: &'c Config) -> Result<Self> {
        Ok(Self {
            client,
            _config: config,
        })
    }

    /// Layer over [`RegistryClient::get_records`] that caches the result.
    ///
    /// It takes [`ManifestDependency`] instead of [`PackageName`] to allow performing some
    /// optimizations by pre-filtering index records on cache-level.
    #[tracing::instrument(level = "trace", skip_all)]
    pub async fn get_records_with_cache(
        &self,
        dependency: &ManifestDependency,
    ) -> Result<IndexRecords> {
        match self.client.get_records(dependency.name.clone()).await? {
            RegistryResource::NotFound => {
                trace!("package not found in registry, pruning cache");
                bail!("package not found in registry: {dependency}")
            }
            RegistryResource::InCache => {
                trace!("getting records from cache");
                todo!()
            }
            RegistryResource::Download { resource, .. } => {
                trace!("got new records, invalidating cache");
                Ok(resource)
            }
        }
    }

    /// Layer over [`RegistryClient::download`] that caches the result.
    #[tracing::instrument(level = "trace", skip_all)]
    pub async fn download_with_cache(&self, package: PackageId) -> Result<PathBuf> {
        match self.client.download(package).await? {
            RegistryResource::NotFound => {
                trace!("archive not found in registry, pruning cache");
                bail!("could not find downloadable archive for package indexed in registry: {package}")
            }
            RegistryResource::InCache => {
                trace!("using cached archive");
                todo!()
            }
            RegistryResource::Download { resource, .. } => {
                trace!("got new archive, invalidating cache");
                Ok(resource)
            }
        }
    }
}
