use anyhow::Result;

use scarb::core::Config;
use scarb::ops::{self, PackageOpts, PublishOpts};

use crate::args::PublishArgs;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: PublishArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let package = args.packages_filter.match_one(&ws)?;
    let index = match args.index {
        Some(index) => index,
        None => package.id.source_id.url.clone(),
    };

    let ops = PublishOpts {
        index_url: index,
        package_opts: PackageOpts {
            allow_dirty: args.shared_args.allow_dirty,
            verify: !args.shared_args.no_verify,
            check_metadata: true,
            features: args.features.try_into()?,
        },
    };

    ops::publish(package.id, &ops, &ws)
}
