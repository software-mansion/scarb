use anyhow::Result;
use std::str::FromStr;
use url::Url;

use scarb::core::Config;
use scarb::core::registry::DEFAULT_REGISTRY_INDEX;
use scarb::ops::{self, PublishDocsOpts};

use crate::args::PublishDocsArgs;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: PublishDocsArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let package = args.packages_filter.match_one(&ws)?;
    let index = match args.index {
        Some(index) => index,
        None => Url::from_str(DEFAULT_REGISTRY_INDEX)?,
    };

    let ops = PublishDocsOpts {
        index_url: index,
        force: args.force,
        allow_dirty: args.allow_dirty,
    };

    ops::publish_docs(package.id, &ops, &ws)
}
