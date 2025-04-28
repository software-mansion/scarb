use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs, iter};

use anyhow::{Result, bail};
use camino::Utf8PathBuf;
use scarb_ui::args::{FeaturesSpec, ToEnvVars};
use tracing::debug;

use scarb_ui::components::Status;

use crate::core::config::get_app_exe_path;
use crate::core::{Config, Package, ScriptDefinition, Workspace};
use crate::internal::fsx::is_executable;
use crate::ops;
use crate::process::exec_replace;
use crate::subcommands::{EXTERNAL_CMD_PREFIX, SCARB_MANIFEST_PATH_ENV, get_env_vars};

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
    let Some(cmd) = find_external_subcommand(cmd, &config.dirs().path_dirs)? else {
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
    features: FeaturesSpec,
) -> Result<()> {
    let package_name = &package.id.name;
    let mut env = HashMap::from_iter([(
        SCARB_MANIFEST_PATH_ENV.into(),
        package.manifest_path().to_string(),
    )]);
    env.extend(features.to_env_vars());
    if let Some(script_definition) = package.manifest.scripts.get("test") {
        debug!("using `test` script: {script_definition}");
        ws.config().ui().print(Status::new(
            "Running",
            &format!("test {package_name} ({script_definition})"),
        ));
        ops::execute_script(script_definition, args, ws, package.root(), Some(env))
    } else {
        debug!("no explicit `test` script found, delegating to scarb-cairo-test");
        ws.config().ui().print(Status::new(
            "Running",
            &format!("cairo-test {package_name}"),
        ));
        let args = args.iter().map(OsString::from).collect::<Vec<_>>();
        let script_definition = ScriptDefinition::new("scarb cairo-test".into());
        ops::execute_script(
            &script_definition,
            args.as_ref(),
            ws,
            package.root(),
            Some(env),
        )
    }
}

// TODO: fix docstring
/// Find an external subcommand executable.
///
/// # Search order
///
/// This function searches for an executable in the following locations, in order:
/// 1. The directory containing the Scarb binary.
/// 2. The directories in the `PATH` environment variable.
/// 3. `{SCARB LOCAL DATA DIR}/bin`.
///
/// Why is the surrounding of the Scarb binary searched for before the `PATH`?
/// Although is sounds tempting to allow users to override Scarb extensions bundled in the default installation,
/// that would cause more harm than good in practice. For example, if the user is working on a custom build of Scarb,
/// but has another one installed globally (for example via ASDF), then their custom build would use global extensions
/// instead of the ones it was built with, which would be very confusing.
fn find_external_subcommand(cmd: &str, path_dirs: &[PathBuf]) -> Result<Option<PathBuf>> {
    let command_exe = format!("{}{}{}", EXTERNAL_CMD_PREFIX, cmd, env::consts::EXE_SUFFIX);

    let exe_path = get_app_exe_path(path_dirs)?;
    let scarb_dir = exe_path
        .parent()
        .expect("Scarb binary path should always have parent directory.");
    let path_dirs = path_dirs.iter().map(PathBuf::as_path);

    Ok(iter::once(scarb_dir)
        .chain(path_dirs)
        .map(|dir| dir.join(&command_exe))
        .find(|file| is_executable(file)))
}

// TODO: fix docstring
/// List all external subcommand executables available in the search paths.
///
/// # Search order
///
/// This function scans, in order:
/// 1. The directory containing the Scarb binary.
/// 2. The directories in the `PATH` environment variable.
/// 3. `{SCARB LOCAL DATA DIR}/bin`.
///
/// For each directory, it collects files whose names start with `EXTERNAL_CMD_PREFIX`
/// and end with the platform suffix, filtering to executables, and returns their paths.
///
/// Why is the surrounding of the Scarb binary searched for before the `PATH`?
/// Although is sounds tempting to allow users to override Scarb extensions bundled in the default installation,
/// that would cause more harm than good in practice. For example, if the user is working on a custom build of Scarb,
/// but has another one installed globally (for example via ASDF), then their custom build would use global extensions
/// instead of the ones it was built with, which would be very confusing.
pub fn list_external_subcommands(path_dirs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let exe_path = get_app_exe_path(path_dirs)?;
    let mut scan_dirs = Vec::new();
    if let Some(parent) = exe_path.parent() {
        scan_dirs.push(parent.to_path_buf());
    }
    scan_dirs.extend(path_dirs.to_owned());

    let mut visited = HashSet::new();
    let mut results = Vec::new();
    for dir in scan_dirs {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with(EXTERNAL_CMD_PREFIX)
                        && name.ends_with(env::consts::EXE_SUFFIX)
                        && is_executable(&path)
                    {
                        // Avoid duplicate paths for given command name
                        let cmd = name
                            .trim_start_matches(EXTERNAL_CMD_PREFIX)
                            .trim_end_matches(env::consts::EXE_SUFFIX);
                        if visited.insert(cmd.to_string()) {
                            results.push(path);
                        }
                    }
                }
            }
        }
    }
    Ok(results)
}
