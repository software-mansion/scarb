use anyhow::Result;

use scarb::core::Config;
use scarb::ops;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(config: &Config) -> Result<()> {
    config.ui().print(ops::list_commands(config).to_string());
    Ok(())
}
