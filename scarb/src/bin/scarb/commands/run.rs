use crate::args::ScriptsRunnerArgs;
use crate::errors::ErrorWithExitCode;
use anyhow::{anyhow, Result};
use indoc::formatdoc;
use itertools::Itertools;
use scarb::core::errors::ScriptExecutionError;
use scarb::core::{Config, Package, PackageName, ScriptDefinition, Workspace};
use scarb::ops;
use scarb_ui::Message;
use serde::{Serialize, Serializer};
use smol_str::SmolStr;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt::Write;
use std::process::ExitCode;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: ScriptsRunnerArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let errors = if args.workspace_root {
        run_for_workspace_root(args, &ws)
            .err()
            .into_iter()
            .collect_vec()
    } else {
        run_for_packages(args, &ws)?
            .into_iter()
            .filter_map(|res| res.err())
            .map(|res| anyhow!(res))
            .collect::<Vec<anyhow::Error>>()
    };
    build_exit_error(errors)
}

fn run_for_workspace_root(args: ScriptsRunnerArgs, ws: &Workspace) -> Result<()> {
    args.script
        .map(|script| {
            let script_definition = ws.script(&script).ok_or_else(|| {
                missing_script_error(&script, "workspace root", " --workspace-root")
            })?;
            ops::execute_script(script_definition, &args.args, ws, ws.root(), None)
        })
        .unwrap_or_else(|| {
            ws.config()
                .ui()
                .print(ScriptsList::for_workspace_root(ws.scripts().clone()));
            Ok(())
        })
}

fn run_for_packages(args: ScriptsRunnerArgs, ws: &Workspace) -> Result<Vec<Result<()>>> {
    Ok(args
        .packages_filter
        .match_many(ws)?
        .into_iter()
        .map(|package| {
            args.script
                .clone()
                .map(|script| run_package_script(script, &args.args, package.clone(), ws))
                .unwrap_or_else(|| {
                    ws.config().ui().print(ScriptsList::for_package(
                        package.id.name.clone(),
                        package.manifest.scripts.clone(),
                        ws.is_single_package(),
                    ));
                    Ok(())
                })
        })
        .collect_vec())
}

fn build_exit_error(errors: Vec<anyhow::Error>) -> Result<()> {
    if errors.is_empty() {
        Ok(())
    } else {
        let exit_code = errors
            .iter()
            .filter_map(|err| {
                err.downcast_ref::<ScriptExecutionError>()
                    .map(|err| err.exit_code)
            })
            .take(1)
            .collect_vec();
        let exit_code = exit_code.first().cloned().unwrap_or(ExitCode::FAILURE);
        let msg = errors
            .into_iter()
            .map(|err| err.to_string())
            .collect::<Vec<String>>()
            .join("\n");
        Err(ErrorWithExitCode::new(anyhow!(msg), exit_code).into())
    }
}

fn run_package_script(
    script: SmolStr,
    args: &[OsString],
    package: Package,
    ws: &Workspace,
) -> Result<()> {
    let script_definition = package.manifest.scripts.get(&script).ok_or_else(|| {
        let package_name = package.id.name.to_string();
        let package_selector = if ws.is_single_package() {
            String::new()
        } else {
            format!(" -p {package_name}")
        };
        missing_script_error(
            &script,
            &format!("package: {package_name}"),
            &package_selector,
        )
    })?;
    ops::execute_script(script_definition, args, ws, package.root(), None)
}

fn missing_script_error(script: &str, source: &str, selector: &str) -> anyhow::Error {
    anyhow!(formatdoc! {r#"
        missing script `{script}` for {source}

        To see a list of scripts, run:
            scarb run{selector}
    "#})
}

#[derive(Serialize, Debug)]
struct PackageScriptsList {
    package: String,
    scripts: BTreeMap<String, String>,
    single_package: bool,
}

#[derive(Serialize, Debug)]
struct WorkspaceScriptsList {
    scripts: BTreeMap<String, String>,
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
enum ScriptsList {
    ForPackage(PackageScriptsList),
    ForWorkspaceRoot(WorkspaceScriptsList),
}

impl ScriptsList {
    pub fn for_package(
        package: PackageName,
        scripts: BTreeMap<SmolStr, ScriptDefinition>,
        single_package: bool,
    ) -> Self {
        let scripts = scripts
            .iter()
            .map(|(name, definition)| (name.to_string(), definition.to_string()))
            .collect();
        Self::ForPackage(PackageScriptsList {
            package: package.to_string(),
            scripts,
            single_package,
        })
    }

    pub fn for_workspace_root(scripts: BTreeMap<SmolStr, ScriptDefinition>) -> Self {
        let scripts = scripts
            .iter()
            .map(|(name, definition)| (name.to_string(), definition.to_string()))
            .collect();
        Self::ForWorkspaceRoot(WorkspaceScriptsList { scripts })
    }

    fn scripts(&self) -> &BTreeMap<String, String> {
        match self {
            Self::ForPackage(p) => &p.scripts,
            Self::ForWorkspaceRoot(w) => &w.scripts,
        }
    }
}

impl PackageScriptsList {
    pub fn text(&self) -> String {
        let mut text = String::new();
        write!(text, "Scripts available via `scarb run`",).unwrap();
        if !self.single_package {
            write!(text, " for package `{}`", self.package).unwrap();
        }
        writeln!(text, ":",).unwrap();
        write!(text, "{}", write_scripts(&self.scripts)).unwrap();
        text
    }
}

impl WorkspaceScriptsList {
    pub fn text(&self) -> String {
        let mut text = String::new();
        writeln!(
            text,
            "Scripts available via `scarb run` for workspace root:",
        )
        .unwrap();
        write!(text, "{}", write_scripts(&self.scripts)).unwrap();
        text
    }
}

fn write_scripts(scripts: &BTreeMap<String, String>) -> String {
    let mut text = String::new();
    for (name, definition) in scripts {
        writeln!(text, "{:<22}: {}", name, definition).unwrap();
    }
    text
}

impl Message for ScriptsList {
    fn text(self) -> String {
        match self {
            Self::ForPackage(p) => p.text(),
            Self::ForWorkspaceRoot(w) => w.text(),
        }
    }

    fn structured<S: Serializer>(self, ser: S) -> Result<S::Ok, S::Error> {
        self.scripts().serialize(ser)
    }
}
