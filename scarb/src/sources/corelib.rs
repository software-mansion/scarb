use std::fmt;

use anyhow::{ensure, Result};
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use rust_embed::RustEmbed;

use crate::core::config::Config;
use crate::core::manifest::{ManifestDependency, Summary};
use crate::core::package::{Package, PackageId};
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
        let registry_dir = self.config.dirs().registry_dir("core");
        fsx::create_dir_all(&registry_dir)?;

        // TODO(mkaput): Include hash part here.
        // TODO(mkaput): Locking.
        let extracted_path = registry_dir.join("core");

        if extracted_path.exists() {
            fsx::remove_dir_all(&extracted_path)?;
        }

        fsx::create_dir_all(&extracted_path)?;

        for path in Corelib::iter() {
            let full_path = extracted_path.join(path.as_ref());
            let data = Corelib::get(path.as_ref()).unwrap().data;
            fsx::create_dir_all(full_path.parent().unwrap())?;
            fsx::write(full_path, data)?;
        }

        let root = extracted_path.join(MANIFEST_FILE_NAME);
        ops::read_package_with_source_id(&root, SourceId::for_core())
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
