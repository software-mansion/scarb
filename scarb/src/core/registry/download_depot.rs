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
        let registry_dir = self.dirs.registry_dir();
        let category_dir = registry_dir.child(category);
        let extracted_path = category_dir.child(package_key);

        if extracted_path.path_unchecked().exists() {
            fsx::remove_dir_all(extracted_path.path_unchecked())?;
        }

        downloader(extracted_path.path_existent()?)?;

        Ok(extracted_path.path_unchecked().to_path_buf())
    }
}
