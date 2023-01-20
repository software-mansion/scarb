use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};

use crate::core::Config;
use crate::internal::fsx;

pub fn download_package_to_cache(
    category: &str,
    package_key: &str,
    config: &Config,
    downloader: impl FnOnce(&Utf8Path) -> Result<()>,
) -> Result<Utf8PathBuf> {
    // TODO(mkaput): Computing checksum.
    let registry_dir = config.dirs().registry_dir();
    let category_dir = registry_dir.child(category);
    let extracted_path = category_dir.child(package_key);

    if extracted_path.path_unchecked().exists() {
        fsx::remove_dir_all(extracted_path.path_unchecked())?;
    }

    downloader(extracted_path.path_existent()?)?;

    Ok(extracted_path.path_unchecked().to_path_buf())
}
