use anyhow::{Ok, Result};

use scarb::core::Config;
use scarb_ui::components::ValueMessage;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(config: &Config) -> Result<()> {
    let parent_fs = config.dirs().cache_dir.parent();
    let path = parent_fs.path_unchecked();
    config.ui().print(ValueMessage::new("path", &path));
    Ok(())
}
