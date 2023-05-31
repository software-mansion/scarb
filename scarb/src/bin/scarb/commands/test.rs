use anyhow::Result;

use scarb::core::Config;
use scarb::ops;

use crate::args::TestArgs;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: TestArgs, config: &Config) -> Result<()> {
    ops::execute_test_subcommand(&args.args, config)
}
