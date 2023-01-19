use anyhow::Result;

use scarb::core::Config;
use scarb::metadata::{Metadata, MetadataOptions};
use scarb::ops;
use scarb::ui::MachineMessage;

use crate::args::MetadataArgs;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: MetadataArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;

    let opts = MetadataOptions {
        version: args.format_version,
        no_deps: args.no_deps,
    };

    let metadata = Metadata::collect(&ws, &opts)?;

    config.ui().print(MachineMessage(metadata));

    Ok(())
}
