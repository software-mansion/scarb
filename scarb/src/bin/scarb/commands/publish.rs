use anyhow::Result;

use scarb::core::Config;
use scarb::ops;
use scarb::ops::PublishOpts;

use crate::args::PublishArgs;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: PublishArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let package = args.packages_filter.match_one(&ws)?;

    let ops = PublishOpts {
        index_url: args.index,
    };

    ops::publish(package.id, &ops, &ws)
}
