use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;
use std::{env, iter};

use anyhow::{bail, Result};
use camino::Utf8PathBuf;
use tracing::debug;

use scarb_ui::components::Status;

use crate::core::{Config, Package, ScriptDefinition, Workspace};
use crate::internal::fsx::is_executable;
use crate::ops::{self, FeaturesSelector};
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
    let Some(cmd) = find_external_subcommand(cmd, config)? else {
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
    features: ops::FeaturesOpts,
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
/// Although is sounds tempting to allow users to override Scarb extensions bundled in the default installation,
/// that would cause more harm than good in practice. For example, if the user is working on a custom build of Scarb,
/// but has another one installed globally (for example via ASDF), then their custom build would use global extensions
/// instead of the ones it was built with, which would be very confusing.
fn find_external_subcommand(cmd: &str, config: &Config) -> Result<Option<PathBuf>> {
    let command_exe = format!("{}{}{}", EXTERNAL_CMD_PREFIX, cmd, env::consts::EXE_SUFFIX);

    let scarb_dir = config
        .app_exe()?
        .parent()
        .expect("Scarb binary path should always have parent directory.");

    let path_dirs = config.dirs().path_dirs.iter().map(AsRef::as_ref);

    Ok(iter::once(scarb_dir)
        .chain(path_dirs)
        .map(|dir| dir.join(&command_exe))
        .find(|file| is_executable(file)))
}

pub trait ToEnv {
    fn to_env_vars(&self) -> HashMap<String, String>;
}

impl ToEnv for ops::FeaturesOpts {
    fn to_env_vars(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();
        match &self.features {
            FeaturesSelector::AllFeatures => {
                env.insert("SCARB_ALL_FEATURES".into(), true.to_string());
            }
            FeaturesSelector::Features(features) if !features.is_empty() => {
                env.insert("SCARB_FEATURES".into(), features.join(","));
            }
            _ => {}
        };
        env.insert(
            "SCARB_NO_DEFAULT_FEATURES".into(),
            self.no_default_features.to_string(),
        );
        env
    }
}
