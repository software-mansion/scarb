use std::fmt;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use once_cell::sync::OnceCell;

use crate::core::config::Config;
use crate::core::manifest::{ManifestDependency, Summary};
use crate::core::package::{Package, PackageId};
use crate::core::source::{Source, SourceId};
use crate::ops;
use crate::MANIFEST_FILE_NAME;

/// This source will only return the package at precisely the `path` specified,
/// and it will be an error if there is not a package at `path`.
pub struct PathSource<'c> {
    source_id: SourceId,
    config: &'c Config,
    packages: OnceCell<Vec<Package>>,
}

impl<'c> PathSource<'c> {
    pub fn new(source_id: SourceId, config: &'c Config) -> Self {
        assert!(source_id.is_path(), "path sources cannot be remote");

        Self {
            source_id,
            config,
            packages: OnceCell::new(),
        }
    }

    pub fn preloaded(packages: &[Package], config: &'c Config) -> Self {
        assert!(
            !packages.is_empty(),
            "PathSource must be preloaded with non-empty package set"
        );

        for wnd in packages.windows(2) {
            let source_a = wnd[0].id.source_id;
            let source_b = wnd[1].id.source_id;
            assert_eq!(
                source_a, source_b,
                "PathSource must be preloaded with packages from the same source"
            );
        }

        Self {
            packages: OnceCell::from(packages.to_vec()),
            ..Self::new(packages[0].id.source_id, config)
        }
    }

    fn ensure_loaded(&self) -> Result<&Vec<Package>> {
        self.packages
            .get_or_try_init(|| Self::read_packages(self.source_id, self.config))
    }

    fn read_packages(source_id: SourceId, config: &Config) -> Result<Vec<Package>> {
        let root = source_id
            .to_path()
            .expect("this has to be a path source ID")
            .join(MANIFEST_FILE_NAME);
        let ws = ops::read_workspace_with_source_id(&root, source_id, config)?;
        Ok(ws.members().collect())
    }
}

#[async_trait]
impl<'c> Source for PathSource<'c> {
    fn source_id(&self) -> SourceId {
        self.source_id
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn query(&mut self, dependency: &ManifestDependency) -> Result<Vec<Summary>> {
        Ok(self
            .ensure_loaded()?
            .iter()
            .map(|pkg| pkg.manifest.summary.clone())
            .filter(|summary| dependency.matches_summary(summary))
            .collect())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn download(&mut self, id: PackageId) -> Result<Package> {
        self.ensure_loaded()?
            .iter()
            .find(|pkg| pkg.id == id)
            .cloned()
            .ok_or_else(|| anyhow!("failed to find {id} in path source {}", self.source_id))
    }
}

impl<'c> fmt::Debug for PathSource<'c> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PathSource")
            .field("source", &self.source_id.to_string())
            .finish_non_exhaustive()
    }
}
