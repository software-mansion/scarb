use std::fmt;
use std::ops::Deref;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use smol::lock::OnceCell;

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
    packages: PackagesCell,
}

impl<'c> PathSource<'c> {
    pub fn new(source_id: SourceId, config: &'c Config) -> Self {
        assert!(source_id.is_path(), "path sources cannot be remote");

        Self {
            source_id,
            config,
            packages: PackagesCell::new(Self::fetch_workspace_at_root),
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

        let source_id = packages[0].id.source_id;

        Self {
            source_id,
            config,
            packages: PackagesCell::preloaded(packages.to_vec()),
        }
    }

    async fn packages(&self) -> Result<&[Package]> {
        self.packages.try_get(self.source_id, self.config).await
    }

    fn fetch_workspace_at_root(source_id: SourceId, config: &Config) -> Result<Vec<Package>> {
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
    #[tracing::instrument(level = "trace", skip(self))]
    async fn query(&mut self, dependency: &ManifestDependency) -> Result<Vec<Summary>> {
        Ok(self
            .packages()
            .await?
            .iter()
            .map(|pkg| pkg.manifest.summary.clone())
            .filter(|summary| dependency.matches_summary(summary))
            .collect())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn download(&mut self, id: PackageId) -> Result<Package> {
        self.packages()
            .await?
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

type PackagesScanner = dyn Fn(SourceId, &Config) -> Result<Vec<Package>> + Send + Sync;

struct PackagesCell {
    cell: OnceCell<Vec<Package>>,
    scanner: Option<Box<PackagesScanner>>,
}

impl PackagesCell {
    fn new(
        scanner: impl Fn(SourceId, &Config) -> Result<Vec<Package>> + Send + Sync + 'static,
    ) -> Self {
        Self {
            cell: OnceCell::new(),
            scanner: Some(Box::new(scanner)),
        }
    }

    fn preloaded(packages: Vec<Package>) -> Self {
        Self {
            cell: OnceCell::from(packages),
            scanner: None,
        }
    }

    async fn try_get(&self, source_id: SourceId, config: &Config) -> Result<&[Package]> {
        self.cell
            .get_or_try_init(|| async {
                // FIXME: Technically one should wrap `f` call in `smol::unblock` in order to avoid
                //   blocking async executor. But quick local benchmarks on our test suite at the
                //   time of writing this actually pointed out that this slows them down by few %.
                //   In the future, it is possible that `smol::unblock` may actually help, or this
                //   has to be debunked with proper benchmarks.
                let f = self.scanner.as_ref().unwrap().deref();
                f(source_id, config)
            })
            .await
            .map(|v| v.as_slice())
    }
}
