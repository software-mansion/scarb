use std::env;

use anyhow::Result;

use scarb::core::Config;
use scarb::ops;

use crate::args::InitArgs;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: InitArgs, config: &Config) -> Result<()> {
    ops::init_package(
        ops::InitOptions {
            name: args.name,
            path: env::current_dir()?,
        },
        config,
    )?;
    println!("Created package.");
    Ok(())
}
