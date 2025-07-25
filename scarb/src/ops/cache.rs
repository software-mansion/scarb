use crate::core::Config;
use crate::internal::fsx;
use anyhow::{Context, Result};

#[tracing::instrument(skip_all, level = "debug")]
pub fn cache_clean(config: &Config) -> Result<()> {
    let parent_fs = config.dirs().cache_dir.parent();
    let path = parent_fs.path_unchecked();

    if path.exists() {
        let _lock = config.tokio_handle().block_on(
            parent_fs
                .advisory_lock(".package-cache.lock", "global Scarb cache", config)
                .acquire_async(),
        )?;
        fsx::remove_dir_all(path).context("failed to clean cache")?;
    }
    Ok(())
}
