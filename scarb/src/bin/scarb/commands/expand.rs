use anyhow::Result;
use smol_str::ToSmolStr;

use crate::args::ExpandArgs;
use scarb::core::{Config, TargetKind};
use scarb::ops;
use scarb::ops::ExpandOpts;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: ExpandArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let package = args.packages_filter.match_one(&ws)?;
    let opts = ExpandOpts {
        features: args.features.try_into()?,
        ugly: args.ugly,
        target_name: args.target_name.map(|n| n.to_smolstr()),
        target_kind: args.target_kind.map(TargetKind::try_new).transpose()?,
    };
    ops::expand(package, opts, &ws)
}
