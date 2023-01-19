use anyhow::Result;

use scarb::core::Config;
use scarb::ops;

use crate::args::NewArgs;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: NewArgs, config: &Config) -> Result<()> {
    let result = ops::new_package(
        ops::InitOptions {
            name: args.init.name,
            path: args.path,
        },
        config,
    )?;

    config
        .ui()
        .print(format!("Created `{}` package.", result.name));
    Ok(())
}
