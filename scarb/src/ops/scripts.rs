use std::collections::HashMap;
use std::ffi::OsString;
use std::rc::Rc;

use anyhow::Result;
use deno_task_shell::{ExecutableCommand, ShellCommand};

use crate::core::errors::ScriptExecutionError;
use crate::core::manifest::ScriptDefinition;
use crate::core::Workspace;
use crate::subcommands::{get_env_vars, EnvVars};

/// Execute user defined script.
pub fn execute_script(
    script_definition: &ScriptDefinition,
    args: &[OsString],
    env_vars: Option<EnvVars>,
    ws: &Workspace<'_>,
) -> Result<()> {
    let env_vars = get_env_vars(ws.config())?
        .into_iter()
        .chain(env_vars.unwrap_or_default().into_iter())
        .map(|(k, v)| {
            (
                k.to_string_lossy().to_string(),
                v.to_string_lossy().to_string(),
            )
        })
        .collect();
    let current_package = ws.current_package()?;
    let cwd = current_package.root();
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
