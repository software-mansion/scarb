use std::collections::HashMap;
use std::ops::DerefMut;

use anyhow::Result;
use itertools::Itertools;
use tracing::trace;

use crate::core::source::Source;
#[cfg(doc)]
use crate::core::Workspace;
use crate::core::{Config, ManifestDependency, Package, PackageId, SourceId, Summary};
use crate::sources::PathSource;

pub mod cache;

/// Source of information about a group of packages.
pub struct Registry<'c> {
    config: &'c Config,
    sources: HashMap<SourceId, Box<dyn Source + 'c>>,
}

impl<'c> Registry<'c> {
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
            let source: Box<dyn Source + 'c> = Box::new(source);
            (source_id, source)
        });
        let sources = HashMap::from_iter(sources);

        Self { config, sources }
    }

    fn ensure_loaded(&mut self, source_id: SourceId) -> Result<&mut (dyn Source + 'c)> {
        // We can't use Entry API here because `load` usage of &self conflicts with it.
        #[allow(clippy::map_entry)]
        if !self.sources.contains_key(&source_id) {
            let source = self.load(source_id)?;
            self.sources.insert(source_id, source);
        }

        Ok(self.sources.get_mut(&source_id).unwrap().deref_mut())
    }

    fn load(&self, source_id: SourceId) -> Result<Box<dyn Source + 'c>> {
        trace!("loading source: {source_id}");
        source_id.load(self.config)
    }

    /// Attempt to find the packages that match a dependency request.
    pub async fn query(&mut self, dependency: &ManifestDependency) -> Result<Vec<Summary>> {
        let source = self.ensure_loaded(dependency.source_id)?;
        source.query(dependency).await
    }

    /// Fetch full package by its ID.
    pub async fn download(&mut self, package_id: PackageId) -> Result<Package> {
        let source = self.ensure_loaded(package_id.source_id)?;
        source.download(package_id).await
    }
}
