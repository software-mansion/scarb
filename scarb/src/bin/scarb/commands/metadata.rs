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

    let features = ops::FeaturesOpts {
        features: args.features,
        all_features: args.all_features,
        no_default_features: args.no_default_features,
    };

    let metadata = ops::collect_metadata(&opts, &ws, features)?;

    config.ui().print(MachineMessage(metadata));

    Ok(())
}
