use std::fmt;

use anyhow::{Context, Result};
use async_trait::async_trait;
use include_dir::{include_dir, Dir};
use tokio::sync::OnceCell;
use tracing::trace;

use crate::core::config::Config;
use crate::core::manifest::{ManifestDependency, Summary};
use crate::core::package::{Package, PackageId};
use crate::core::source::Source;
use crate::core::SourceId;
use crate::sources::PathSource;

/// Serves Cairo standard library packages.
pub struct StandardLibSource<'c> {
    config: &'c Config,
    path_source: OnceCell<PathSource<'c>>,
}

impl<'c> StandardLibSource<'c> {
    pub fn new(config: &'c Config) -> Self {
        Self {
            config,
            path_source: OnceCell::new(),
        }
    }

    async fn ensure_loaded(&self) -> Result<&PathSource<'c>> {
        self.path_source.get_or_try_init(|| self.load()).await
    }

    #[tracing::instrument(name = "standard_lib_source_load", level = "trace", skip(self))]
    async fn load(&self) -> Result<PathSource<'c>> {
        static CORE: Dir<'_> = include_dir!("$SCARB_CORE_PATH");

        let tag = core_version_tag();

        let registry_fs = self.config.dirs().registry_dir();
        let std_fs = registry_fs.child("std");
        let tag_fs = std_fs.child(&tag);
        let tag_path = tag_fs.path_existent()?;

        if !tag_fs.is_ok() {
            trace!("extracting Cairo standard library: {tag}");
            let _lock = self.config.package_cache_lock().acquire_async().await?;

            unsafe {
                tag_fs.recreate()?;
            }

            let core_fs = tag_fs.child("core");
            CORE.extract(core_fs.path_existent()?)
                .context("failed to extract Cairo standard library")?;

            tag_fs.mark_ok()?;
        }

        Ok(PathSource::recursive_at(
            tag_path,
            SourceId::for_std(),
            self.config,
        ))
    }
}

#[async_trait]
impl<'c> Source for StandardLibSource<'c> {
    #[tracing::instrument(level = "trace", skip(self))]
    async fn query(&self, dependency: &ManifestDependency) -> Result<Vec<Summary>> {
        self.ensure_loaded().await?.query(dependency).await
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn download(&self, package_id: PackageId) -> Result<Package> {
        self.ensure_loaded().await?.download(package_id).await
    }
}

impl<'c> fmt::Debug for StandardLibSource<'c> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StandardLibSource").finish_non_exhaustive()
    }
}

fn core_version_tag() -> String {
    let core_version_info = crate::version::get().cairo;
    core_version_info
        .commit_info
        .map(|commit| {
            assert!(!commit.short_commit_hash.starts_with('v'));
            commit.short_commit_hash
        })
        .unwrap_or_else(|| format!("v{}", core_version_info.version))
}
