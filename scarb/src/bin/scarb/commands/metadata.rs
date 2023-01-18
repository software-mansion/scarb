use anyhow::{Context, Result};

use scarb::core::Config;
use scarb::metadata::{Metadata, MetadataOptions};
use scarb::ops;

use crate::args::MetadataArgs;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: MetadataArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;

    let opts = MetadataOptions {
        version: args.format_version,
        no_deps: args.no_deps,
    };

    let metadata = Metadata::collect(&ws, &opts)?;

    let json = serde_json::to_string_pretty(&metadata).context("Failed to produce JSON output.")?;
    println!("{json}");

    Ok(())
}
