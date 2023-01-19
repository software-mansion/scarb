use anyhow::Result;

use scarb::core::Config;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(config: &Config) -> Result<()> {
    config.ui().print("Installed commands:");
    todo!("not implemented yet.")
}
