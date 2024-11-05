use anyhow::Result;

use crate::args::BuildArgs;
use scarb::core::Config;
use scarb::ops;
use scarb::ops::CompileOpts;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: BuildArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let packages = args
        .packages_filter
        .match_many(&ws)?
        .into_iter()
        .map(|p| p.id)
        .collect::<Vec<_>>();
    let opts = CompileOpts::try_new(
        args.features,
        args.ignore_cairo_version,
        args.test,
        args.target_names,
        args.target_kinds,
    )?;
    ops::compile(packages, opts, &ws)
}
