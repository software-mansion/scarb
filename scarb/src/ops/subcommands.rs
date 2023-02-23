use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Result};

use crate::core::Config;
use crate::process::{exec_replace, is_executable};
use crate::{EXTERNAL_CMD_PREFIX, SCARB_ENV};

#[tracing::instrument(level = "debug", skip(config))]
pub fn execute_external_subcommand(cmd: &str, args: &[&OsStr], config: &Config) -> Result<()> {
    let Some(cmd) = find_external_subcommand(cmd, config) else {
        // TODO(mkaput): Reuse clap's no such command message logic here.
        bail!("no such command: `{cmd}`");
    };

    // TODO(mkaput): Jobserver.
    // TODO(mkaput): Write a test that CTRL+C kills everything, like Cargo's death,
    //   but perhaps use an external bash script? Use Job Objects or smth else to fix it.

    let mut cmd = Command::new(cmd);
    cmd.args(args);
    cmd.env(SCARB_ENV, config.app_exe()?);
    cmd.env("PATH", config.dirs().path_env());
    cmd.env("SCARB_CACHE", config.dirs().cache_dir.path_unchecked());
    cmd.env("SCARB_CONFIG", config.dirs().config_dir.path_unchecked());
    cmd.env("SCARB_TARGET_DIR", config.target_dir().path_unchecked());
    cmd.env("SCARB_MANIFEST_PATH", config.manifest_path());
    cmd.env("SCARB_UI_VERBOSITY", config.ui().verbosity().to_string());
    cmd.env("SCARB_LOG", config.log_filter_directive());
    exec_replace(&mut cmd)
}

fn find_external_subcommand(cmd: &str, config: &Config) -> Option<PathBuf> {
    let command_exe = format!("{EXTERNAL_CMD_PREFIX}{cmd}{}", env::consts::EXE_SUFFIX);
    config
        .dirs()
        .path_dirs
        .iter()
        .map(|dir| dir.join(&command_exe))
        .find(|file| is_executable(file))
}
