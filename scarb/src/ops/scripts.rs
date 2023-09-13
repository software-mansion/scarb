use std::collections::HashMap;
use std::ffi::OsString;
use std::rc::Rc;

use anyhow::Result;
use camino::Utf8Path;
use deno_task_shell::{ExecutableCommand, ShellCommand};

use crate::core::errors::ScriptExecutionError;
use crate::core::manifest::ScriptDefinition;
use crate::core::Workspace;
use crate::subcommands::get_env_vars;

/// Execute user defined script.
pub fn execute_script(
    script_definition: &ScriptDefinition,
    args: &[OsString],
    ws: &Workspace<'_>,
    cwd: &Utf8Path,
    custom_env: Option<HashMap<OsString, OsString>>,
) -> Result<()> {
    let env_vars = get_env_vars(ws)?
        .into_iter()
        .map(|(k, v)| {
            (
                k.to_string_lossy().to_string(),
                v.to_string_lossy().to_string(),
            )
        })
        .chain(custom_env.unwrap_or_default().into_iter().map(|(k, v)| {
            (
                k.to_string_lossy().to_string(),
                v.to_string_lossy().to_string(),
            )
        }))
        .collect();
    let custom_commands = HashMap::from([
        // Used to ensure deno_task_shell scripts use the current scarb executable.
        (
            "scarb".to_string(),
            Rc::new(ExecutableCommand::new(
                ws.config().app_exe()?.display().to_string(),
            )) as Rc<dyn ShellCommand>,
        ),
    ]);
    let list = script_definition.parse(args)?;

    let runtime = ws.config().tokio_handle();
    let exit_code = runtime.block_on(deno_task_shell::execute(
        list,
        env_vars,
        (&cwd).as_ref(),
        custom_commands,
    ));

    if exit_code != 0 {
        Err(ScriptExecutionError::new(exit_code).into())
    } else {
        Ok(())
    }
}
