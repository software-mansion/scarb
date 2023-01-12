use std::ffi::OsString;

use anyhow::{anyhow, Result};

use scarb::core::Config;
use scarb::ops::execute_external_subcommand;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: Vec<OsString>, config: &Config) -> Result<()> {
    assert!(!args.is_empty());
    let cmd = &args[0]
        .clone()
        .into_string()
        .map_err(|_| anyhow!("command name must be valid UTF-8"))?;
    let args = &args.iter().skip(1).map(AsRef::as_ref).collect::<Vec<_>>();
    execute_external_subcommand(cmd, args, config)
}
