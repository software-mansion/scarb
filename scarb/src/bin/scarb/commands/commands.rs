use anyhow::Result;

use crate::args::ScarbArgs;
use scarb::core::Config;
use scarb::ops;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(config: &Config) -> Result<()> {
    let mut builtins = ScarbArgs::get_subcommands();
    config
        .ui()
        .print(ops::list_commands(config, &mut builtins).to_string());
    Ok(())
}
