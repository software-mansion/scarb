use anyhow::Result;

use crate::args::FmtArgs;
use crate::errors::error_with_exit_code;
use scarb::core::Config;
use scarb::ops::{self, FmtAction};

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: FmtArgs, config: &Config) -> Result<()> {
    // The action the formatted should perform,
    // e.g. check formatting, format in place, or emit formatted file to stdout.
    let action = if args.check {
        // adding the `--check` flag will shortcircuit the ability to emit the formatted file
        FmtAction::Check
    } else if let Some(emit_target) = args.emit {
        FmtAction::Emit(emit_target)
    } else {
        // Format in place is the default option
        FmtAction::Fix
    };
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
            action,
            color: !args.no_color,
        },
        &ws,
    )? {
        Ok(())
    } else {
        error_with_exit_code(1)
    }
}
