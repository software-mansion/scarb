use std::collections::BTreeMap;
use std::path::PathBuf;
use std::{env, fmt, fs};

use anyhow::Result;
use serde::{Serialize, Serializer};

use scarb::core::Config;
use scarb::process::is_executable;
use scarb::EXTERNAL_CMD_PREFIX;
use scarb_ui::Message;

use crate::args::ScarbArgs;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Serialize)]
enum CommandInfo {
    BuiltIn { about: Option<String> },
    External { path: PathBuf },
}

impl fmt::Display for CommandInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandInfo::BuiltIn { about: Some(about) } => write!(f, "{}", about),
            CommandInfo::BuiltIn { about: None } => write!(f, "",),
            CommandInfo::External { path } => write!(f, "{}", path.display()),
        }
    }
}

#[derive(Serialize, Debug)]
struct CommandsList {
    commands: BTreeMap<String, CommandInfo>,
}

impl Message for CommandsList {
    fn text(self) -> String {
        let mut text = String::from("Installed Commands:\n");
        for (name, info) in self.commands {
            text.push_str(&format!("{:<22}: {}\n", name, info));
        }
        text
    }

    fn structured<S: Serializer>(self, ser: S) -> Result<S::Ok, S::Error> {
        self.commands.serialize(ser)
    }
}

fn list_commands(config: &Config, builtins: &BTreeMap<String, Option<String>>) -> CommandsList {
    let prefix = EXTERNAL_CMD_PREFIX;
    let suffix = env::consts::EXE_SUFFIX;

    // Directory containing the Scarb executable.
    let scarb_exe_dir = config
        .app_exe()
        .ok()
        .and_then(|p| p.parent())
        .map(PathBuf::from);
    let mut commands = BTreeMap::new();
    for dir in config.dirs().path_dirs.iter().chain(scarb_exe_dir.iter()) {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let Some(filename) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
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

    // In case of name conflict, builtin commands take precedence.
    let mut builtin_commands = builtins
        .iter()
        .map(|(name, about)| {
            (
                name.clone(),
                CommandInfo::BuiltIn {
                    about: about.clone(),
                },
            )
        })
        .collect();
    commands.append(&mut builtin_commands);

    CommandsList { commands }
}

#[tracing::instrument(skip_all, level = "info")]
pub fn run(config: &Config) -> Result<()> {
    let builtins = ScarbArgs::get_builtin_subcommands();
    config.ui().print(list_commands(config, &builtins));
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::env;
    use std::path::PathBuf;

    use assert_fs::prelude::*;
    use assert_fs::TempDir;
    use camino::Utf8Path;

    use scarb::core::Config;
    use scarb::process::make_executable;

    use super::{list_commands, CommandInfo};

    #[test]
    fn cmd_list() {
        let t = TempDir::new().unwrap();

        let cache_dir = TempDir::new().unwrap();
        let config_dir = TempDir::new().unwrap();
        let manifest = t.child("Scarb.toml");
        let path_dir = t.child("bin");
        let sub_cmd = path_dir.child(format!("scarb-hello{}", env::consts::EXE_SUFFIX));
        sub_cmd.touch().unwrap();
        make_executable(&sub_cmd);

        let config = Config::builder(Utf8Path::from_path(manifest.path()).unwrap().to_path_buf())
            .global_cache_dir_override(Some(Utf8Path::from_path(&cache_dir).unwrap().to_path_buf()))
            .global_config_dir_override(Some(
                Utf8Path::from_path(&config_dir).unwrap().to_path_buf(),
            ))
            .path_env_override(Some(vec![PathBuf::from(
                Utf8Path::from_path(&path_dir).unwrap().to_path_buf(),
            )]))
            .build()
            .unwrap();

        let mut cmd = list_commands(&config, &BTreeMap::new());

        assert_eq!(
            cmd.commands.remove("hello").unwrap(),
            CommandInfo::External {
                path: sub_cmd.path().to_path_buf()
            }
        );
    }
}
