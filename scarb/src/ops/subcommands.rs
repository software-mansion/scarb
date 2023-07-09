use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Result};
use tracing::debug;

use crate::core::{Config, Package, Workspace};
use crate::ops;
use crate::process::{exec_replace, is_executable};
use crate::subcommands::{get_env_vars, EXTERNAL_CMD_PREFIX};
use crate::ui::Status;

#[tracing::instrument(level = "debug", skip(config))]
pub fn execute_external_subcommand(cmd: &str, args: &[OsString], config: &Config) -> Result<()> {
    let Some(cmd) = find_external_subcommand(cmd, config) else {
        // TODO(mkaput): Reuse clap's no such command message logic here.
        bail!("no such command: `{cmd}`");
    };

    // TODO(mkaput): Jobserver.
    // TODO(#129): Write a test that CTRL+C kills everything, like Cargo's death,
    //   but perhaps use an external bash script? Use Job Objects or smth else to fix it.

    let mut cmd = Command::new(cmd);
    cmd.args(args);
    cmd.envs(get_env_vars(config)?);

    exec_replace(&mut cmd)
}

#[tracing::instrument(level = "debug", skip(config))]
pub fn execute_test_subcommand(
    package: &Package,
    args: &[OsString],
    config: &Config,
    ws: &Workspace<'_>,
) -> Result<()> {
    config.ui().print(Status::new(
        "Running tests",
        format!("for package: {}", package.id.name).as_str(),
    ));
    if let Some(script_definition) = package.manifest.scripts.get("test") {
        debug!("using `test` script: {script_definition}");
        ops::execute_script(script_definition, args, package.root(), ws)
    } else {
        debug!("no explicit `test` script found, delegating to scarb-cairo-test");
        let args = args.iter().map(OsString::from).collect::<Vec<_>>();
        execute_external_subcommand("cairo-test", args.as_ref(), config)
    }
}

fn find_external_subcommand(cmd: &str, config: &Config) -> Option<PathBuf> {
    let command_exe = format!("{}{}{}", EXTERNAL_CMD_PREFIX, cmd, env::consts::EXE_SUFFIX);
    let mut dirs = config.dirs().path_dirs.clone();

    // Add directory containing the Scarb executable.
    if let Ok(path) = config.app_exe() {
        if let Some(parent) = path.parent() {
            let path = PathBuf::from(parent);
            dirs.push(path);
        }
    }

    dirs.iter()
        .map(|dir| dir.join(&command_exe))
        .find(|file| is_executable(file))
}
