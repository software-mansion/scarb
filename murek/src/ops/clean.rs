use anyhow::{Context, Result};

use crate::core::Config;
use crate::internal::fsx;

#[tracing::instrument(skip_all, level = "debug")]
pub fn clean(config: &Config) -> Result<()> {
    fsx::remove_dir_all(config.target_dir.as_unchecked())
        .context("failed to clean generated artifacts")
}
