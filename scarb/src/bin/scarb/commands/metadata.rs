use anyhow::Result;

use scarb::core::Config;
use scarb::ops;
use scarb_ui::components::MachineMessage;

use crate::args::MetadataArgs;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: MetadataArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;

    let opts = ops::MetadataOptions {
        version: args.format_version,
        no_deps: args.no_deps,
    };

    let metadata = ops::collect_metadata(&opts, &ws)?;

    config.ui().print(MachineMessage(metadata));

    Ok(())
}
