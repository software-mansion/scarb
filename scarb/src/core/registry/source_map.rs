use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use itertools::Itertools;
use tokio::sync::RwLock;
use tracing::trace;

use crate::core::registry::Registry;
use crate::core::source::Source;
#[cfg(doc)]
use crate::core::Workspace;
use crate::core::{Config, ManifestDependency, Package, PackageId, SourceId, Summary};
use crate::sources::PathSource;

/// Source of information about a group of packages.
pub struct SourceMap<'c> {
    config: &'c Config,
    sources: RwLock<HashMap<SourceId, Arc<dyn Source + 'c>>>,
}

impl<'c> SourceMap<'c> {
    /// Preload the registry with already loaded [`Package`]s.
    ///
    /// For example, a [`Workspace`] may load packages during construction/parsing/early phases
    /// for various operations, and this preload step avoids doubly-loading and
    /// parsing packages on the filesystem by inserting them all into the registry
    /// with their in-memory formats.
    pub fn preloaded(packages: impl Iterator<Item = Package>, config: &'c Config) -> Self {
        let sources = packages
            .sorted_by_key(|pkg| pkg.id.source_id)
            .group_by(|pkg| pkg.id.source_id);
        let sources = sources.into_iter().map(|(source_id, packages)| {
            let packages = packages.collect::<Vec<_>>();
            let source = PathSource::preloaded(&packages, config);
            let source: Arc<dyn Source + 'c> = Arc::new(source);
            (source_id, source)
        });
        let sources = RwLock::new(HashMap::from_iter(sources));
        Self { config, sources }
    }

    async fn ensure_loaded(&self, source_id: SourceId) -> Result<Arc<dyn Source + 'c>> {
        let loaded_source = self.sources.read().await.get(&source_id).cloned();
        if let Some(source) = loaded_source {
            Ok(source)
        } else {
            trace!("loading source: {source_id}");
            let source = source_id.load(self.config)?;
            self.sources.write().await.insert(source_id, source.clone());
            Ok(source)
        }
    }
}

#[async_trait(?Send)]
impl<'c> Registry for SourceMap<'c> {
    /// Attempt to find the packages that match a dependency request.
    async fn query(&self, dependency: &ManifestDependency) -> Result<Vec<Summary>> {
        let source = self.ensure_loaded(dependency.source_id).await?;
        source.query(dependency).await
    }

    /// Fetch full package by its ID.
    async fn download(&self, package_id: PackageId) -> Result<Package> {
        let source = self.ensure_loaded(package_id.source_id).await?;
        source.download(package_id).await
    }
}
