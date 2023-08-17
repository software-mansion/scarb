use std::env;

use anyhow::{Error, Result};
use clap::Parser;
use tracing::debug;
use tracing_log::AsTrace;
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

fn main() {
    let args = ScarbArgs::parse();

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(args.verbose.log_level_filter().as_trace().into())
                .with_env_var("SCARB_LOG")
                .from_env_lossy(),
        )
        .init();

    // Pre-create Ui used in error reporting, because we will move `args` to `cli_main`.
    let ui = Ui::new(args.ui_verbosity(), args.output_format());

    if let Err(err) = cli_main(args) {
        exit_with_error(err, &ui);
    }
}

fn exit_with_error(err: Error, ui: &Ui) {
    debug!("exit_with_error; err={:?}", err);

    if let Some(ErrorWithExitCode { source, exit_code }) = err.downcast_ref::<ErrorWithExitCode>() {
        if let Some(source_err) = source {
            ui.anyhow(source_err);
        }
        std::process::exit(*exit_code);
    } else if let Some(ScriptExecutionError { exit_code }) =
        err.downcast_ref::<ScriptExecutionError>()
    {
        std::process::exit(*exit_code);
    } else {
        ui.anyhow(&err);
        std::process::exit(1);
    }
}

fn cli_main(args: ScarbArgs) -> Result<()> {
    let ui_verbosity = args.ui_verbosity();
    let ui_output_format = args.output_format();

    let manifest_path = ops::find_manifest_path(args.manifest_path.as_deref())?;

    let mut config = Config::builder(manifest_path)
        .global_cache_dir_override(args.global_cache_dir)
        .global_config_dir_override(args.global_config_dir)
        .target_dir_override(args.target_dir)
        .ui_verbosity(ui_verbosity)
        .ui_output_format(ui_output_format)
        .offline(args.offline)
        .log_filter_directive(env::var_os("SCARB_LOG"))
        .profile(args.profile_spec.determine()?)
        .build()?;

    commands::run(args.command, &mut config)
}
