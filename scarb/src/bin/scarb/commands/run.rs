use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt::Write;

use anyhow::{anyhow, Result};
use indoc::formatdoc;
use serde::{Serialize, Serializer};
use smol_str::SmolStr;

use scarb::core::{Config, Package, Workspace};
use scarb::ops;
use scarb::ui::Message;

use crate::args::ScriptsRunnerArgs;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: ScriptsRunnerArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let package = args.packages_filter.match_one(&ws)?;
    if let Some(script) = args.script {
        run_script(script, args.args, package, &ws, config)
    } else {
        list_scripts(package, config)
    }
}

fn run_script(
    script: SmolStr,
    args: Vec<OsString>,
    package: Package,
    ws: &Workspace,
    config: &Config,
) -> Result<()> {
    let script_definition = package.manifest.scripts.get(&script).ok_or_else(|| {
        anyhow!(formatdoc! {r#"
            missing script `{script}`

            To see a list of scripts, run:
                scarb run
            "#})
    })?;
    ops::execute_script(script_definition, &args, ws, config)
}

fn list_scripts(package: Package, config: &Config) -> Result<()> {
    let scripts = package
        .manifest
        .scripts
        .iter()
        .map(|(name, definition)| (name.to_string(), definition.to_string()))
        .collect();
    config.ui().print(ScriptsList(scripts));
    Ok(())
}

#[derive(Serialize, Debug)]
struct ScriptsList(BTreeMap<String, String>);

impl Message for ScriptsList {
    fn text(self) -> String {
        let mut text = String::new();
        writeln!(text, "Scripts available via `scarb run`:").unwrap();
        for (name, definition) in self.0 {
            writeln!(text, "{:<20}: {}", name, definition).unwrap();
        }
        text
    }

    fn structured<S: Serializer>(self, ser: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(ser)
    }
}
