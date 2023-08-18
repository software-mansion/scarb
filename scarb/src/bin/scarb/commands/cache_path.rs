use anyhow::{Ok, Result};

use scarb::core::Config;
use scarb_ui::components::ValueMessage;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(config: &Config) -> Result<()> {
    let path = config.dirs().cache_dir.path_unchecked();
    config.ui().print(ValueMessage::new("path", &path));
    Ok(())
}
