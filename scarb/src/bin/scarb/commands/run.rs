use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt::Write;

use anyhow::{anyhow, Result};
use indoc::formatdoc;
use serde::{Serialize, Serializer};
use smol_str::SmolStr;

use scarb::core::errors::ScriptExecutionError;
use scarb::core::{Config, Package, Workspace};
use scarb::ops;
use scarb_ui::Message;

use crate::args::ScriptsRunnerArgs;
use crate::errors::ErrorWithExitCode;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: ScriptsRunnerArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let packages = args.packages_filter.match_many(&ws)?;
    let errors = packages
        .into_iter()
        .map(|package| {
            args.script
                .clone()
                .map(|script| run_script(script, &args.args, package.clone(), &ws))
                .unwrap_or_else(|| list_scripts(package, &ws))
        })
        .filter_map(|res| res.err())
        .map(|res| anyhow!(res))
        .collect::<Vec<anyhow::Error>>();
    if errors.is_empty() {
        Ok(())
    } else {
        let exit_code = errors
            .iter()
            .filter_map(|err| {
                err.downcast_ref::<ScriptExecutionError>()
                    .map(|err| err.exit_code)
            })
            .find(|exit_code| *exit_code != 0)
            .unwrap_or(1);
        let msg = errors
            .into_iter()
            .map(|err| err.to_string())
            .collect::<Vec<String>>()
            .join("\n");
        Err(ErrorWithExitCode::new(anyhow!(msg), exit_code).into())
    }
}

fn run_script(script: SmolStr, args: &[OsString], package: Package, ws: &Workspace) -> Result<()> {
    let script_definition = package.manifest.scripts.get(&script).ok_or_else(|| {
        let package_name = package.id.name.to_string();
        let package_selector = if !ws.is_single_package() {
            format!(" -p {package_name}")
        } else {
            String::new()
        };
        anyhow!(formatdoc! {r#"
            missing script `{script}` for package: {package_name}

            To see a list of scripts, run:
                scarb run{package_selector}
            "#})
    })?;
    ops::execute_script(script_definition, args, ws, package.root(), None)
}

fn list_scripts(package: Package, ws: &Workspace) -> Result<()> {
    let scripts = package
        .manifest
        .scripts
        .iter()
        .map(|(name, definition)| (name.to_string(), definition.to_string()))
        .collect();
    let package = package.id.name.to_string();
    let single_package = ws.is_single_package();
    ws.config().ui().print(ScriptsList {
        scripts,
        package,
        single_package,
    });
    Ok(())
}

#[derive(Serialize, Debug)]
struct ScriptsList {
    package: String,
    scripts: BTreeMap<String, String>,
    single_package: bool,
}

impl Message for ScriptsList {
    fn text(self) -> String {
        let mut text = String::new();
        write!(text, "Scripts available via `scarb run`",).unwrap();
        if !self.single_package {
            write!(text, " for package `{}`", self.package).unwrap();
        }
        writeln!(text, ":",).unwrap();
        for (name, definition) in self.scripts {
            writeln!(text, "{:<22}: {}", name, definition).unwrap();
        }
        text
    }

    fn structured<S: Serializer>(self, ser: S) -> Result<S::Ok, S::Error> {
        self.scripts.serialize(ser)
    }
}
