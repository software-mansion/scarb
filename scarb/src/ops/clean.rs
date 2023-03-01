use anyhow::{Context, Result};

use crate::core::Config;
use crate::internal::fsx;

#[tracing::instrument(skip_all, level = "debug")]
pub fn clean(config: &Config) -> Result<()> {
    let path = config.target_dir().path_unchecked();
    fsx::remove_dir_all(path).context("failed to clean generated artifacts")
}
