use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Result};
use camino::Utf8PathBuf;
use tracing::debug;

use scarb_ui::components::Status;

use crate::core::{Config, Package, ScriptDefinition, Workspace};
use crate::internal::fsx::is_executable;
use crate::ops;
use crate::process::exec_replace;
use crate::subcommands::{get_env_vars, EXTERNAL_CMD_PREFIX, SCARB_MANIFEST_PATH_ENV};

/// Prepare environment and execute an external subcommand.
///
/// NOTE: This may replace the current process.
#[tracing::instrument(level = "debug", skip(config))]
pub fn execute_external_subcommand(
    cmd: &str,
    args: &[OsString],
    custom_env: Option<HashMap<OsString, OsString>>,
    config: &Config,
    target_dir: Option<Utf8PathBuf>,
) -> Result<()> {
    let Some(cmd) = find_external_subcommand(cmd, config) else {
        // TODO(mkaput): Reuse clap's no such command message logic here.
        bail!("no such command: `{cmd}`");
    };

    // TODO(mkaput): Jobserver.

    let mut cmd = Command::new(cmd);
    cmd.args(args);
    cmd.envs(get_env_vars(config, target_dir)?);
    if let Some(env) = custom_env {
        cmd.envs(env);
    }

    exec_replace(&mut cmd)
}

#[tracing::instrument(level = "debug", skip(ws))]
pub fn execute_test_subcommand(
    package: &Package,
    args: &[OsString],
    ws: &Workspace<'_>,
) -> Result<()> {
    let package_name = &package.id.name;
    let env = Some(HashMap::from_iter([(
        SCARB_MANIFEST_PATH_ENV.into(),
        package.manifest_path().to_string(),
    )]));
    if let Some(script_definition) = package.manifest.scripts.get("test") {
        debug!("using `test` script: {script_definition}");
        ws.config().ui().print(Status::new(
            "Running",
            &format!("test {package_name} ({script_definition})"),
        ));
        ops::execute_script(script_definition, args, ws, package.root(), env)
    } else {
        debug!("no explicit `test` script found, delegating to scarb-cairo-test");
        ws.config().ui().print(Status::new(
            "Running",
            &format!("cairo-test {package_name}"),
        ));
        let args = args.iter().map(OsString::from).collect::<Vec<_>>();
        let script_definition = ScriptDefinition::new("scarb cairo-test".into());
        ops::execute_script(&script_definition, args.as_ref(), ws, package.root(), env)
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
