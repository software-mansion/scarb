use std::collections::HashSet;
use std::fmt;
use std::path::PathBuf;

use anyhow::{anyhow, bail, ensure, Context, Result};
use async_trait::async_trait;
use tracing::trace;

use scarb_ui::components::Status;

use crate::core::registry::client::http::HttpRegistryClient;
use crate::core::registry::client::local::LocalRegistryClient;
use crate::core::registry::client::RegistryClient;
use crate::core::registry::index::IndexRecord;
use crate::core::registry::package_source_store::PackageSourceStore;
use crate::core::source::Source;
use crate::core::{
    Config, DependencyVersionReq, ManifestDependency, Package, PackageId, SourceId, Summary,
    TargetKind,
};
use crate::sources::PathSource;

pub struct RegistrySource<'c> {
    source_id: SourceId,
    config: &'c Config,
    client: Box<dyn RegistryClient + 'c>,
    package_sources: PackageSourceStore<'c>,
}

impl<'c> RegistrySource<'c> {
    pub fn new(source_id: SourceId, config: &'c Config) -> Result<Self> {
        let client = Self::create_client(source_id, config)?;

        // TODO(mkaput): Wrap remote clients in a disk caching layer.
        // TODO(mkaput): Wrap all clients in an in-memory caching layer.

        let package_sources = PackageSourceStore::new(source_id, config);

        Ok(Self {
            source_id,
            config,
            client,
            package_sources,
        })
    }

    pub fn create_client(
        source_id: SourceId,
        config: &'c Config,
    ) -> Result<Box<dyn RegistryClient + 'c>> {
        assert!(source_id.is_registry());
        match source_id.url.scheme() {
            "file" => {
                trace!("creating local registry client for: {source_id}");
                let path = source_id
                    .url
                    .to_file_path()
                    .map_err(|_| anyhow!("url is not a valid path: {}", source_id.url))?;
                Ok(Box::new(LocalRegistryClient::new(&path)?))
            }
            "http" | "https" => {
                trace!("creating http registry client for: {source_id}");
                Ok(Box::new(HttpRegistryClient::new(source_id, config)?))
            }
            _ => {
                bail!("unsupported registry protocol: {source_id}")
            }
        }
    }
}

#[async_trait]
impl<'c> Source for RegistrySource<'c> {
    #[tracing::instrument(level = "trace", skip(self))]
    async fn query(&self, dependency: &ManifestDependency) -> Result<Vec<Summary>> {
        let Some(records) = self
            .client
            .get_records(dependency.name.clone())
            .await
            .with_context(|| {
                format!(
                    "failed to lookup for `{dependency}` in registry: {}",
                    self.source_id
                )
            })?
        else {
            bail!("package not found in registry: {dependency}");
        };

        let build_summary_from_index_record = |record: &IndexRecord| {
            let package_id = PackageId::new(
                dependency.name.clone(),
                record.version.clone(),
                self.source_id,
            );

            let dependencies = record
                .dependencies
                .iter()
                .map(|index_dep| {
                    ManifestDependency::builder()
                        .name(index_dep.name.clone())
                        .version_req(DependencyVersionReq::from(index_dep.req.clone()))
                        .source_id(self.source_id)
                        .build()
                })
                .collect();

            Summary::builder()
                .package_id(package_id)
                .dependencies(dependencies)
                .target_kinds(HashSet::from_iter([TargetKind::LIB]))
                .no_core(record.no_core)
                .build()
        };

        // TODO(mkaput): Save checksums out-of-band for later use.

        Ok(records
            .iter()
            // NOTE: We filter based on IndexRecords here, to avoid unnecessarily allocating
            //   PackageIds just to abandon them soon after.
            .filter(|record| dependency.version_req.matches(&record.version))
            .map(build_summary_from_index_record)
            .collect())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn download(&self, id: PackageId) -> Result<Package> {
        let is_downloaded = self.client.is_downloaded(id).await;

        ensure!(
            self.config.network_allowed() || self.client.is_offline() || is_downloaded,
            "cannot download from `{}` in offline mode",
            self.source_id
        );

        if !is_downloaded && !self.client.is_offline() {
            self.config
                .ui()
                .print(Status::new("Downloading", &id.to_string()));
        }

        let archive = self.client.download(id).await?;

        self.verify_checksum(id, archive.clone()).await?;
        self.load_package(id, archive).await
    }
}

impl<'c> RegistrySource<'c> {
    async fn verify_checksum(&self, id: PackageId, _archive: PathBuf) -> Result<()> {
        self.config
            .ui()
            .verbose(Status::new("Verifying", &id.to_string()));

        // TODO(mkaput): Verify checksum.

        Ok(())
    }

    /// Turn the downloaded `.tar.zst` tarball into a [`Package`].
    ///
    /// This method extracts the tarball into cache directory, and then loads it using
    /// suitably configured [`PathSource`].
    async fn load_package(&self, id: PackageId, archive: PathBuf) -> Result<Package> {
        if self.client.is_offline() {
            self.config
                .ui()
                .print(Status::new("Unpacking", &id.to_string()));
        } else {
            self.config
                .ui()
                .verbose(Status::new("Unpacking", &id.to_string()));
        }

        let path = self.package_sources.extract(id, archive).await?;
        let path_source = PathSource::recursive_at(&path, self.source_id, self.config);
        path_source.download(id).await
    }
}

impl<'c> fmt::Debug for RegistrySource<'c> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RegistrySource")
            .field("source", &self.source_id.to_string())
            .finish_non_exhaustive()
    }
}
