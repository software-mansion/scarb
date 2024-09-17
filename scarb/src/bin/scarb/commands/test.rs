use crate::args::TestArgs;
use anyhow::Result;
use itertools::Itertools;
use scarb::core::Config;
use scarb::ops;
use scarb::ops::{validate_features, FeaturesOpts};

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: TestArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let packages = args
        .packages_filter
        .match_many(&ws)?
        .into_iter()
        .collect_vec();
    let features_opts: FeaturesOpts = args.features.clone().try_into()?;
    validate_features(&packages, &features_opts)?;
    packages.iter().try_for_each(|package| {
        ops::execute_test_subcommand(package, &args.args, &ws, args.features.clone()).map(|_| ())
    })
}
