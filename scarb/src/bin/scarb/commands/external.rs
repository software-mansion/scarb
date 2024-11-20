use std::ffi::OsString;

use anyhow::{anyhow, Result};
use camino::Utf8PathBuf;
use scarb::core::Config;
use scarb::ops;
use scarb::ops::execute_external_subcommand;
use scarb_ui::{Ui, Verbosity};

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: Vec<OsString>, config: &mut Config) -> Result<()> {
    let target_dir = get_target_dir(config)?;

    let Some((cmd, args)) = args.split_first() else {
        panic!("`args` should never be empty.")
    };

    let cmd = cmd
        .to_str()
        .ok_or_else(|| anyhow!("command name must be valid UTF-8"))?;

    // NOTE: This may replace the current process.
    execute_external_subcommand(cmd, args, None, config, target_dir)
}

fn get_target_dir(config: &mut Config) -> Result<Option<Utf8PathBuf>> {
    let original_ui = config.ui();
    let muted_ui = Ui::new(Verbosity::Quiet, original_ui.output_format());
    config.set_ui(muted_ui);
    let target_dir = if config.manifest_path().exists() {
        let ws = ops::read_workspace(config.manifest_path(), config)?;
        Some(ws.target_dir().path_unchecked().to_owned())
    } else {
        None
    };
    config.set_ui(original_ui);
    Ok(target_dir)
}
