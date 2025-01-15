use scarb::ops::{self, LintOptions};

use crate::args::LintArgs;
use anyhow::Result;
use scarb::core::Config;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: LintArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let packages = args
        .packages_filter
        .match_many(&ws)?
        .into_iter()
        .collect::<Vec<_>>();

    ops::lint(
        LintOptions {
            packages,
            test: args.test,
            fix: args.fix,
            ignore_cairo_version: args.ignore_cairo_version,
        },
        &ws,
    )
}
