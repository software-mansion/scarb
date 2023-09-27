use anyhow::Result;

use crate::args::BuildArgs;
use scarb::core::{Config, TargetKind};
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
    let exclude_targets: Vec<TargetKind> = if args.test {
        Vec::new()
    } else {
        vec![TargetKind::TEST.clone()]
    };
    let opts = CompileOpts { exclude_targets };
    ops::compile(packages, opts, &ws)
}
