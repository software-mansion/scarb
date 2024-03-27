use anyhow::Result;

use scarb::core::Config;
use scarb::ops;

use crate::args::TestArgs;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: TestArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;

    args.packages_filter
        .match_many(&ws)?
        .iter()
        .try_for_each(|package| ops::execute_test_subcommand(package, &args.args, &ws).map(|_| ()))
}
