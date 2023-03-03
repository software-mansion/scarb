use anyhow::Result;

use crate::args::FmtArgs;
use crate::errors::ErrorWithExitCode;
use scarb::core::Config;
use scarb::ops;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: FmtArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    if ops::format(
        ops::FmtOptions {
            check: args.check,
            pkg_name: args.package,
            color: !args.no_color,
        },
        &ws,
    )? {
        Ok(())
    } else {
        Err(ErrorWithExitCode::code(1).into())
    }
}
