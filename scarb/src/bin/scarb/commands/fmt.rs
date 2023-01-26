use anyhow::Result;

use crate::args::FmtArgs;
use scarb::core::Config;
use scarb::ops;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: FmtArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    match ops::format(
        ops::FmtOptions {
            check: args.check,
            pkg_name: args.package,
            color: !args.no_color,
        },
        &ws,
    ) {
        Ok(true) => Ok(()),
        _ => std::process::exit(1),
    }
}
