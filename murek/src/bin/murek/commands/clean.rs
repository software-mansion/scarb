use anyhow::Result;
use murek::core::Config;

use murek::ops;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(config: &Config) -> Result<()> {
    ops::clean(config)
}
