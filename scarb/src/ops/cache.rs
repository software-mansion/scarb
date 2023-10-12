use anyhow::{Context, Result};

use crate::core::Config;
use crate::internal::asyncx::block_on;
use crate::internal::fsx;

#[tracing::instrument(skip_all, level = "debug")]
pub fn cache_clean(config: &Config) -> Result<()> {
    let path = config.dirs().cache_dir.path_unchecked();
    if path.exists() {
        let _lock = block_on(config, config.package_cache_lock().acquire_async())?;
        fsx::remove_dir_all(path).context("failed to clean cache")?;
    }
    Ok(())
}
