use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fmt::Display;
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

use anyhow::{bail, Result};

use crate::core::Config;
use crate::process::{exec_replace, is_executable};
use crate::SCARB_ENV;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum CommandInfo {
    External { path: PathBuf },
}

impl Display for CommandInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandInfo::External { path } => write!(f, "{}", path.display()),
        }
    }
}

pub struct CommandsList {
    pub commands: BTreeMap<String, CommandInfo>,
}

impl Display for CommandsList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Installed Commands:")?;
        for (name, info) in self.commands.iter() {
            writeln!(f, "{name}: {info}")?;
        }
        Ok(())
    }
}

#[tracing::instrument(level = "debug", skip(config))]
pub fn list_commands(config: &Config) -> CommandsList {
    let prefix = "scarb-";
    let suffix = env::consts::EXE_SUFFIX;

    let mut commands = BTreeMap::new();
    for dir in config.dirs().path_dirs.iter() {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            _ => continue,
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let filename = match path.file_name().and_then(|s| s.to_str()) {
                Some(filename) => filename,
                _ => continue,
            };
            if !filename.starts_with(prefix) || !filename.ends_with(suffix) {
                continue;
            }
            if is_executable(entry.path()) {
                let end = filename.len() - suffix.len();
                commands.insert(
                    filename[prefix.len()..end].to_string(),
                    CommandInfo::External { path: path.clone() },
                );
            }
        }
    }

    CommandsList { commands }
}

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
    let command_exe = format!("scarb-{}{}", cmd, env::consts::EXE_SUFFIX);
    config
        .dirs()
        .path_dirs
        .iter()
        .map(|dir| dir.join(&command_exe))
        .find(|file| is_executable(file))
}
