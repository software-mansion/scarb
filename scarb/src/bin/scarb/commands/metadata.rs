use anyhow::Result;

use scarb::core::Config;
use scarb::ops;
use scarb_ui::components::MachineMessage;

use crate::args::MetadataArgs;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: MetadataArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;

    let features = args.features.try_into()?;
    let opts = ops::MetadataOptions {
        version: args.format_version,
        no_deps: args.no_deps,
        features,
        ignore_cairo_version: args.ignore_cairo_version,
    };

    let metadata = ops::collect_metadata(&opts, &ws)?;

    config.ui().force_print(MachineMessage(metadata));

    Ok(())
}
