use std::ffi::OsString;

use anyhow::{anyhow, Result};

use scarb::core::Config;
use scarb::ops;
use scarb::ops::execute_external_subcommand;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: Vec<OsString>, config: &Config) -> Result<()> {
    let target_dir = if config.manifest_path().exists() {
        let ws = ops::read_workspace(config.manifest_path(), config)?;
        Some(ws.target_dir().path_unchecked().to_owned())
    } else {
        None
    };

    let Some((cmd, args)) = args.split_first() else {
        panic!("`args` should never be empty.")
    };

    let cmd = cmd
        .to_str()
        .ok_or_else(|| anyhow!("command name must be valid UTF-8"))?;

    // NOTE: This may replace the current process.
    execute_external_subcommand(cmd, args, None, config, target_dir)
}
