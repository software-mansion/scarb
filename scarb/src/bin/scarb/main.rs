use std::env;
use std::process::ExitCode;
use std::str::FromStr;

use anyhow::{Error, Result};
use clap::Parser;
use tracing::debug;
use tracing_subscriber::EnvFilter;

use args::ScarbArgs;
use scarb::core::errors::ScriptExecutionError;
use scarb::core::Config;
use scarb::ops;
use scarb_ui::Ui;

use crate::errors::ErrorWithExitCode;

mod args;
mod commands;
mod errors;
mod interactive;

fn main() -> ExitCode {
    let args = ScarbArgs::parse();

    // Pre-create Ui used in logging & error reporting, because we will move `args` to `cli_main`.
    let ui = Ui::new(args.verbose.clone().into(), args.output_format());

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(
                    FromStr::from_str(args.verbose.as_trace().as_str()).unwrap(),
                )
                .with_env_var("SCARB_LOG")
                .from_env_lossy(),
        )
        .with_ansi(ui.has_colors_enabled_stderr())
        .init();

    if let Err(err) = cli_main(args) {
        return exit_with_error(err, &ui);
    }

    ExitCode::SUCCESS
}

fn exit_with_error(err: Error, ui: &Ui) -> ExitCode {
    debug!("exit_with_error; err={:?}", err);

    if let Some(ErrorWithExitCode { source, exit_code }) = err.downcast_ref::<ErrorWithExitCode>() {
        if let Some(source_err) = source {
            ui.anyhow(source_err);
        }
        *exit_code
    } else if let Some(ScriptExecutionError { exit_code }) =
        err.downcast_ref::<ScriptExecutionError>()
    {
        *exit_code
    } else {
        ui.anyhow(&err);
        ExitCode::FAILURE
    }
}

fn cli_main(args: ScarbArgs) -> Result<()> {
    let ui_output_format = args.output_format();
    let scarb_log = env::var_os("SCARB_LOG").unwrap_or_else(|| args.verbose.as_trace().into());

    let manifest_path = ops::find_manifest_path(args.manifest_path.as_deref())?;

    let mut config = Config::builder(manifest_path)
        .global_cache_dir_override(args.global_cache_dir)
        .global_config_dir_override(args.global_config_dir)
        .target_dir_override(args.target_dir)
        .ui_verbosity(args.verbose.clone().into())
        .ui_output_format(ui_output_format)
        .offline(args.offline)
        .log_filter_directive(Some(scarb_log))
        .profile(args.profile_spec.determine()?)
        .build()?;

    commands::run(args.command, &mut config)
}
