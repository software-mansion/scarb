use std::env;

use anyhow::{anyhow, Result};
use camino::Utf8PathBuf;

use scarb::core::Config;
use scarb::ops;

use crate::args::InitArgs;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: InitArgs, config: &Config) -> Result<()> {
    let path = Utf8PathBuf::from_path_buf(env::current_dir()?)
        .map_err(|path| anyhow!("path `{}` is not UTF-8 encoded", path.display()))?;

    ops::init_package(
        ops::InitOptions {
            name: args.name,
            path,
        },
        config,
    )?;
    config.ui().print("Created package.");
    Ok(())
}
