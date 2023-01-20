use std::fmt;

use anyhow::{ensure, Result};
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use rust_embed::RustEmbed;

use crate::core::config::Config;
use crate::core::manifest::{ManifestDependency, Summary};
use crate::core::package::{Package, PackageId};
use crate::core::registry::download::download_package_to_cache;
use crate::core::source::{Source, SourceId};
use crate::core::MANIFEST_FILE_NAME;
use crate::internal::fsx;
use crate::ops;

#[derive(RustEmbed)]
#[folder = "../corelib"]
struct Corelib;

pub struct CorelibSource<'c> {
    config: &'c Config,
    package: OnceCell<Package>,
}

impl<'c> CorelibSource<'c> {
    pub fn new(config: &'c Config) -> Self {
        Self {
            config,
            package: OnceCell::new(),
        }
    }

    fn ensure_loaded(&mut self) -> Result<Package> {
        self.package.get_or_try_init(|| self.load()).cloned()
    }

    fn load(&self) -> Result<Package> {
        // TODO(mkaput): Include core version or hash part here.
        let root = download_package_to_cache("core", "core", self.config, |tmp| {
            for path in Corelib::iter() {
                let full_path = tmp.join(path.as_ref());
                let data = Corelib::get(path.as_ref()).unwrap().data;
                fsx::create_dir_all(full_path.parent().unwrap())?;
                fsx::write(full_path, data)?;
            }

            Ok(())
        })?;

        let manifest_path = root.join(MANIFEST_FILE_NAME);
        ops::read_package_with_source_id(&manifest_path, SourceId::for_core())
    }
}

#[async_trait]
impl<'c> Source for CorelibSource<'c> {
    fn source_id(&self) -> SourceId {
        SourceId::for_core()
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn query(&mut self, dependency: &ManifestDependency) -> Result<Vec<Summary>> {
        let package = self.ensure_loaded()?;
        if dependency.matches_summary(&package.manifest.summary) {
            Ok(vec![package.manifest.summary.clone()])
        } else {
            Ok(Vec::new())
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn download(&mut self, package_id: PackageId) -> Result<Package> {
        let package = self.ensure_loaded()?;
        ensure!(package.id == package_id, "unknown package {package_id}");
        Ok(package)
    }
}

impl<'c> fmt::Debug for CorelibSource<'c> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CorelibSource").finish_non_exhaustive()
    }
}
