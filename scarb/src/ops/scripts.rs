use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::process::ExitCode;
use std::rc::Rc;

use anyhow::Result;
use camino::Utf8Path;
use deno_task_shell::{ExecutableCommand, KillSignal, ShellCommand, parser};
use scarb_ui::components::Status;

use crate::core::{Package, Workspace};
use crate::core::errors::ScriptExecutionError;
use crate::core::manifest::ScriptDefinition;
use crate::subcommands::get_env_vars;

/// Execute a magic script if it exists in the package manifest.
///
/// Magic scripts are special script names that are automatically executed by Scarb
/// at specific points in the build process (e.g., "build", "package").
pub fn execute_magic_script_if_exists(
    script_name: &str,
    package: &Package,
    ws: &Workspace<'_>,
) -> Result<()> {
    let Some(script_definition) = package.manifest.scripts.get(script_name) else {
        return Ok(());
    };
    
    // Ensure no two instances of Scarb will run this script at the same time.
    let _guard = ws.target_dir().child("scarb").advisory_lock(
        &format!(".scarb-{script_name}.lock"),
        &format!("{script_name} script"),
        ws.config(),
    );
    
    ws.config().ui().print(Status::new(
        "Running",
        &format!("{script_name} script for {pkg}", pkg=package.id),
    ));
    
    execute_script(script_definition, &[], ws, package.root(), None)
}

/// Execute user defined script.
pub fn execute_script(
    script_definition: &ScriptDefinition,
    args: &[OsString],
    ws: &Workspace<'_>,
    cwd: &Utf8Path,
    custom_env: Option<HashMap<String, String>>,
) -> Result<()> {
    let custom_commands = HashMap::from([
        // Used to ensure deno_task_shell scripts use the current scarb executable.
        (
            "scarb".to_string(),
            Rc::new(ExecutableCommand::new(
                "scarb".to_string(),
                ws.config().app_exe()?.to_path_buf(),
            )) as Rc<dyn ShellCommand>,
        ),
    ]);
    let list = script_definition.parse(args)?;
    let mut env_vars = collect_env(custom_env, ws)?;
    // HACK: We help deno_task_shell use colors ;)
    // We want to avoid the problem of piping the coloured text, by ensuring script contains no pipes.
    // Perhaps there's a better way to tackle this issue (Maybe exec_replace instead of using env vars?).
    if list.items.iter().all(|x| !has_pipe(&x.sequence)) {
        for col_var in ["TERM", "COLORTERM"] {
            if let Ok(value) = env::var(col_var) {
                env_vars.insert(col_var.into(), value);
            }
        }
        env_vars.insert(
            "CLICOLOR".into(),
            ws.config().ui().has_colors_enabled().to_string(),
        );
    }

    let env_vars = env_vars
        .into_iter()
        .map(|(key, value)| (OsString::from(key), OsString::from(value)))
        .collect();

    let runtime = ws.config().tokio_handle();
    let exit_code = runtime.block_on(deno_task_shell::execute(
        list,
        env_vars,
        cwd.to_owned().into(),
        custom_commands,
        KillSignal::default(),
    ));

    if exit_code != 0 {
        let exit_code: ExitCode = u8::try_from(exit_code)
            .map(Into::into)
            .unwrap_or(ExitCode::FAILURE);
        Err(ScriptExecutionError::new(exit_code).into())
    } else {
        Ok(())
    }
}

fn collect_env(
    custom_env: Option<HashMap<String, String>>,
    ws: &Workspace<'_>,
) -> Result<HashMap<String, String>> {
    let target_dir = Some(ws.target_dir().path_unchecked().to_owned());
    let scarb_env = get_env_vars(ws.config(), target_dir)?
        .into_iter()
        .map(|(k, v)| {
            (
                k.to_string_lossy().to_string(),
                v.to_string_lossy().to_string(),
            )
        });
    let env_vars: HashMap<String, String> = std::env::vars()
        .chain(scarb_env)
        .chain(custom_env.unwrap_or_default())
        .collect();
    Ok(env_vars)
}

fn has_pipe(seq: &parser::Sequence) -> bool {
    match seq {
        parser::Sequence::ShellVar(_) => false,
        parser::Sequence::Pipeline(pipeline) => match &pipeline.inner {
            parser::PipelineInner::PipeSequence(_) => true,
            parser::PipelineInner::Command(command) => match &command.inner {
                parser::CommandInner::Simple(_) => false,
                parser::CommandInner::Subshell(subshell) => {
                    subshell.items.iter().map(|x| &x.sequence).any(has_pipe)
                }
            },
        },
        parser::Sequence::BooleanList(list) => has_pipe(&list.current) || has_pipe(&list.next),
    }
}
