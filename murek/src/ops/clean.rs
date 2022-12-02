use anyhow::{Context, Result};

use crate::core::Config;

#[tracing::instrument(skip_all, level = "debug")]
pub fn clean(config: &Config) -> Result<()> {
    config
        .target_dir()?
        .clean()
        .context("failed to clean generated artifacts")
}
