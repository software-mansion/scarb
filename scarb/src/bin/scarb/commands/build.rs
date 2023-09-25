use anyhow::Result;

use crate::args::BuildArgs;
use scarb::core::{Config, Target, TargetKind};
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
        vec![Target::TEST.into()]
    };
    let opts = CompileOpts { exclude_targets };
    ops::compile(packages, opts, &ws)
}
