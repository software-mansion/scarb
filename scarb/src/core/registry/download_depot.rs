use std::sync::Arc;

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};

use crate::dirs::AppDirs;
use crate::internal::fsx;

#[derive(Debug)]
pub struct DownloadDepot {
    dirs: Arc<AppDirs>,
}

impl DownloadDepot {
    pub fn new(dirs: Arc<AppDirs>) -> Self {
        Self { dirs }
    }

    pub fn get_or_download(
        &self,
        category: &str,
        package_key: &str,
        downloader: impl FnOnce(&Utf8Path) -> Result<()>,
    ) -> Result<Utf8PathBuf> {
        // TODO(mkaput): Locking and computing checksum.
        let registry_dir = self.dirs.registry_dir(category);
        fsx::create_dir_all(&registry_dir)?;

        let extracted_path = registry_dir.join(package_key);

        if extracted_path.exists() {
            fsx::remove_dir_all(&extracted_path)?;
        }

        fsx::create_dir_all(&extracted_path)?;

        downloader(&extracted_path)?;

        Ok(extracted_path)
    }
}
