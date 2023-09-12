use anyhow::{Context, Result};

use crate::core::Config;
use crate::internal::fsx;
use crate::ops;

#[tracing::instrument(skip_all, level = "debug")]
pub fn clean(config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let path = ws.target_dir().path_unchecked();
    if path.exists() {
        fsx::remove_dir_all(path).context("failed to clean generated artifacts")?;
    }
    Ok(())
}
