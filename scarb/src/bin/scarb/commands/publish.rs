use anyhow::Result;

use scarb::core::Config;
use scarb::ops::{self, PackageOpts, PublishOpts};

use crate::args::PublishArgs;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: PublishArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let package = args.packages_filter.match_one(&ws)?;

    let ops = PublishOpts {
        index_url: args.index,
        package_opts: PackageOpts {
            allow_dirty: args.shared_args.allow_dirty,
            verify: !args.shared_args.no_verify,
        },
    };

    ops::publish(package.id, &ops, &ws)
}
