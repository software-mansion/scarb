use std::fmt;
use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use git2::build::RepoBuilder;
use git2::{Error, Repository};
use once_cell::sync::OnceCell;

use crate::core::config::Config;
use crate::core::manifest::{ManifestDependency, Summary};
use crate::core::package::{Package, PackageId};
use crate::core::source::{Source, SourceId};
use crate::core::{GitReference, SourceKind, MANIFEST_FILE_NAME};
use crate::ops;

pub struct CorelibSource<'c> {
    source_id: SourceId,
    config: &'c Config,
    package: OnceCell<Package>,
}

impl<'c> CorelibSource<'c> {
    pub fn new(source_id: SourceId, config: &'c Config) -> Self {
        assert!(
            source_id.is_git(),
            "only remote corelib sources are allowed"
        );

        Self {
            source_id,
            config,
            package: OnceCell::new(),
        }
    }

    fn registry_src_corelib_path(&self) -> PathBuf {
        self.config.dirs.registry_src_dir.join("cairo")
    }

    fn checkout_cairo(&self) -> Result<Repository, Error> {
        let path = self.registry_src_corelib_path();

        let tag = match &self.source_id.kind {
            SourceKind::Git(reference) => match reference {
                GitReference::Tag(tag) => tag,
                _ => panic!("corelib source must be tagged with version"),
            },
            _ => unreachable!(""),
        };

        let mut builder = RepoBuilder::new();
        builder.bare(true);
        builder.branch(tag);

        // Clone the project.
        builder.clone(self.source_id.url.as_str(), path.as_path())
    }

    fn ensure_loaded(&mut self) -> Result<&Package> {
        self.package
            .get_or_try_init(|| self.read_package(self.source_id, self.config))
    }

    fn read_package(&self, source_id: SourceId, config: &Config) -> Result<Package> {
        self.checkout_cairo()
            .expect("failed to checkout cairo project");
        let root = self.registry_src_corelib_path().join(MANIFEST_FILE_NAME);
        let ws = ops::read_workspace_with_source_id(&root, source_id, config)?;
        Ok((*ws.current_package().unwrap()).clone())
    }
}

#[async_trait]
impl<'c> Source for CorelibSource<'c> {
    fn source_id(&self) -> SourceId {
        self.source_id
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn query(&mut self, dependency: &ManifestDependency) -> Result<Vec<Summary>> {
        let package = self.ensure_loaded().unwrap();
        let summary = package.manifest.summary.clone();
        if dependency.matches_summary(&summary) {
            Ok(vec![summary])
        } else {
            Ok(Vec::new())
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn download(&mut self, _id: PackageId) -> Result<Package> {
        Ok(self.ensure_loaded().unwrap().clone())
    }
}

impl<'c> fmt::Debug for CorelibSource<'c> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CorelibSource")
            .field("source", &self.source_id.to_string())
            .finish_non_exhaustive()
    }
}
