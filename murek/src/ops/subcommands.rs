use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use anyhow::{bail, Context, Result};

use crate::core::Config;
use crate::MUREK_ENV;

#[tracing::instrument(level = "debug", skip(config))]
pub fn execute_external_subcommand(cmd: &str, args: &[&OsStr], config: &Config) -> Result<()> {
    let Some(cmd) = find_external_subcommand(cmd, config) else {
        // TODO(mkaput): Reuse clap's no such command message logic here.
        bail!("no such command: `{cmd}`");
    };

    // TODO(mkaput): Jobserver.
    // TODO(mkaput): Write a test that CTRL+C kills everything, like Cargo's death,
    //   but perhaps use an external bash script? Use Job Objects or smth else to fix it.

    let exit_status = Command::new(&cmd)
        .args(args)
        .env(MUREK_ENV, config.app_exe()?)
        .env("PATH", config.dirs.path_env())
        .spawn()
        .with_context(|| format!("failed to spawn subcommand: {}", cmd.display()))?
        .wait()
        .with_context(|| format!("failed to wait for subcommand to finish: {}", cmd.display()))?;

    if exit_status.success() {
        Ok(())
    } else {
        bail!("process exited unsuccessfully: {exit_status}");
    }
}

fn find_external_subcommand(cmd: &str, config: &Config) -> Option<PathBuf> {
    let command_exe = format!("murek-{}{}", cmd, env::consts::EXE_SUFFIX);
    config
        .dirs
        .path_dirs
        .iter()
        .map(|dir| dir.join(&command_exe))
        .find(|file| is_executable(file))
}

#[cfg(unix)]
fn is_executable<P: AsRef<Path>>(path: P) -> bool {
    use std::os::unix::prelude::*;
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(windows)]
fn is_executable<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().is_file()
}
