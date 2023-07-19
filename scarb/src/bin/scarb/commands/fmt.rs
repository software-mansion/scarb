use anyhow::Result;

use crate::args::FmtArgs;
use crate::errors::error_with_exit_code;
use scarb::core::Config;
use scarb::ops;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: FmtArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let packages = args
        .packages_filter
        .match_many(&ws)?
        .into_iter()
        .map(|p| p.id)
        .collect::<Vec<_>>();
    if ops::format(
        ops::FmtOptions {
            packages,
            check: args.check,
            color: !args.no_color,
        },
        &ws,
    )? {
        Ok(())
    } else {
        error_with_exit_code(1)
    }
}
