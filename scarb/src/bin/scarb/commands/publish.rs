use anyhow::Result;
use std::slice;
use std::str::FromStr;
use url::Url;

use scarb::core::Config;
use scarb::core::registry::DEFAULT_REGISTRY_INDEX;
use scarb::ops::{self, PackageOpts, PublishOpts, validate_features};

use crate::args::PublishArgs;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: PublishArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let package = args.packages_filter.match_one(&ws)?;
    let index = match args.index {
        Some(index) => index,
        None => Url::from_str(DEFAULT_REGISTRY_INDEX)?,
    };

    let features_opts = args.features.try_into()?;
    validate_features(slice::from_ref(&package), &features_opts)?;
    let ops = PublishOpts {
        index_url: index,
        package_opts: PackageOpts {
            allow_dirty: args.shared_args.allow_dirty,
            verify: !args.shared_args.no_verify,
            check_metadata: true,
            features: features_opts,
            ignore_cairo_version: args.ignore_cairo_version,
        },
    };

    ops::publish(package.id, &ops, &ws)
}
