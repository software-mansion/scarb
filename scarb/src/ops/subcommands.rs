use std::collections::HashMap;
use std::convert::TryFrom;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs, iter};

use anyhow::{Result, bail};
use camino::Utf8PathBuf;
use scarb_ui::args::{FeaturesSpec, ToEnvVars};
use tracing::debug;

use crate::core::{Config, Package, ScriptDefinition, Workspace};
use crate::internal::fsx::is_executable;
use crate::ops;
use crate::process::exec_replace;
use crate::subcommands::{EXTERNAL_CMD_PREFIX, SCARB_MANIFEST_PATH_ENV, get_env_vars};
use itertools::Itertools;
use scarb_ui::components::Status;

#[derive(Debug)]
pub struct SubcommandDirs {
    pub scarb_exe_dir: PathBuf,
    pub path_dirs: Vec<PathBuf>,
}

impl SubcommandDirs {
    pub fn iter(&self) -> impl Iterator<Item = &Path> {
        iter::once(self.scarb_exe_dir.as_path()).chain(self.path_dirs.iter().map(|p| p.as_path()))
    }
}

impl TryFrom<&Config> for SubcommandDirs {
    type Error = anyhow::Error;

    fn try_from(config: &Config) -> Result<Self, Self::Error> {
        let path_dirs = config.dirs().path_dirs.clone();
        let scarb_exe_dir = config
            .app_exe()?
            .parent()
            .expect("Scarb binary path should always have parent directory.")
            .to_path_buf();
        Ok(Self {
            scarb_exe_dir,
            path_dirs,
        })
    }
}

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
    let subcommand_dirs = SubcommandDirs::try_from(config)?;
    let Some(cmd) = find_external_subcommand(cmd, &subcommand_dirs)? else {
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

    exec_replace(cmd)
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
/// Although it sounds tempting to allow users to override Scarb extensions bundled in the default installation,
/// that would cause more harm than good in practice. For example, if the user is working on a custom build of Scarb,
/// but has another one installed globally (for example via ASDF), then their custom build would use global extensions
/// instead of the ones it was built with, which would be very confusing.
fn find_external_subcommand(
    cmd: &str,
    subcommand_dirs: &SubcommandDirs,
) -> Result<Option<PathBuf>> {
    let command_exe = format!("{}{}{}", EXTERNAL_CMD_PREFIX, cmd, env::consts::EXE_SUFFIX);
    Ok(subcommand_dirs
        .iter()
        .map(|dir| dir.join(&command_exe))
        .find(|file| is_executable(file)))
}

/// Information about an external subcommand.
pub struct ExternalSubcommand {
    /// Name of the subcommand without prefix and postfix.
    pub name: String,
    /// Path to the subcommand executable.
    pub path: PathBuf,
}

/// List all unique external subcommand executables available in the search paths,
/// each represented as an `ExternalSubcommand`.
///
/// # Search order
///
/// This function scans the following locations, in order:
/// 1. The directory containing the Scarb binary.
/// 2. The directories in the `PATH` environment variable.
/// 3. `{SCARB LOCAL DATA DIR}/bin`.
///
/// For each directory, it collects executables whose names start with `EXTERNAL_CMD_PREFIX`
/// and end with the platform suffix, and returns them wrapped in `ExternalSubcommand`.
///
/// If multiple executables with the same subcommand name are found in different directories,
/// only the first one found (according to the search order above) is included in the result.
///
/// Why is the surrounding of the Scarb binary searched for before the `PATH`?
/// Although it sounds tempting to allow users to override Scarb extensions bundled in the default installation,
/// that would cause more harm than good in practice. For example, if the user is working on a custom build of Scarb,
/// but has another one installed globally (for example via ASDF), then their custom build would use global extensions
/// instead of the ones it was built with, which would be very confusing.
pub fn list_external_subcommands(
    subcommand_dirs: &SubcommandDirs,
) -> Result<Vec<ExternalSubcommand>> {
    let prefix = EXTERNAL_CMD_PREFIX;
    let suffix = env::consts::EXE_SUFFIX;

    let subcommands = subcommand_dirs
        .iter()
        .filter_map(|dir| fs::read_dir(dir).ok())
        .flat_map(|entries| entries.flatten())
        .filter(|entry| is_executable(entry.path()))
        .filter_map(|entry| {
            let path = entry.path();
            let basename = path.file_name()?.to_str()?;
            if !basename.starts_with(prefix) || !basename.ends_with(suffix) {
                return None;
            }
            let cmd_name = basename
                .trim_start_matches(prefix)
                .trim_end_matches(suffix)
                .to_string();
            if cmd_name.is_empty() {
                return None;
            }
            Some(ExternalSubcommand {
                name: cmd_name,
                path: path.clone(),
            })
        })
        .unique_by(|cmd| cmd.name.clone())
        .collect::<Vec<_>>();

    Ok(subcommands)
}
