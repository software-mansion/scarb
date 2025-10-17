use anyhow::{Error, Result};
use args::ScarbArgs;
use clap::Parser;
use mimalloc::MiMalloc;
use scarb::core::Config;
use scarb::core::errors::ScriptExecutionError;
use scarb::ops;
use scarb::process::WillExecReplace;
use scarb_ui::Ui;
use scarb_ui::args::VerbositySpec;
use std::env;
use std::process::ExitCode;
use std::str::FromStr;
use tracing::debug;

use crate::errors::ErrorWithExitCode;

mod args;
mod commands;
mod errors;
mod fsx;
mod interactive;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> ExitCode {
    // NOTE: Never ever create droppable objects in `main`.
    match main_that_can_exec_replace() {
        Ok(exit_code) => exit_code,
        Err(err) => err.take_over(),
    }
}

fn main_that_can_exec_replace() -> Result<ExitCode, WillExecReplace> {
    let args = ScarbArgs::parse();

    // Pre-create Ui used in logging and error reporting, because we will move `args` to `cli_main`.
    let ui = Ui::new(args.verbose.clone().into(), args.output_format());

    let _guard = init_logging(args.verbose.clone(), &ui);

    match cli_main(args) {
        Ok(()) => Ok(ExitCode::SUCCESS),
        Err(err) => match err.downcast::<WillExecReplace>() {
            Ok(err) => Err(err),
            Err(err) => Ok(exit_with_error(err, &ui)),
        },
    }
}

fn init_logging(verbose: VerbositySpec, ui: &Ui) -> Option<impl Drop> {
    use chrono::Local;
    use std::fs;

    use std::path::PathBuf;
    use tracing_chrome::ChromeLayerBuilder;
    use tracing_subscriber::filter::{EnvFilter, LevelFilter, Targets};
    use tracing_subscriber::fmt::Layer;
    use tracing_subscriber::fmt::time::Uptime;
    use tracing_subscriber::prelude::*;

    let mut guard = None;

    let fmt_layer = Layer::new()
        .with_writer(std::io::stderr)
        .with_ansi(ui.has_colors_enabled_stderr())
        .with_timer(Uptime::default())
        .with_filter(
            EnvFilter::builder()
                .with_default_directive(FromStr::from_str(verbose.as_trace().as_str()).unwrap())
                .with_env_var("SCARB_LOG")
                .from_env_lossy(),
        )
        .with_filter(cairo_lang_utils::logging::exclude_salsa());

    let tracing_profile = env::var("SCARB_TRACING_PROFILE")
        .ok()
        .map(|var| {
            let s = var.as_str();
            s == "true" || s == "1"
        })
        .unwrap_or(false);

    let profile_layer = if tracing_profile {
        let mut path = PathBuf::from(format!(
            "./scarb-profile-{}.json",
            Local::now().to_rfc3339()
        ));

        // Create the file now, so that we early panic, and `fs::canonicalize` will work.
        let profile_file = fs::File::create(&path).expect("failed to create profile file");

        // Try to canonicalise the path so that it is easier to find the file from logs.
        if let Ok(canonical) = fsx::canonicalize(&path) {
            path = canonical;
        }

        eprintln!(
            "this Scarb run will output tracing profile to: {}",
            path.display()
        );
        eprintln!(
            "open that file with https://ui.perfetto.dev (or chrome://tracing) to analyze it"
        );

        let (profile_layer, profile_layer_guard) = ChromeLayerBuilder::new()
            .writer(profile_file)
            .include_args(true)
            .build();

        // Filter out less important logs because they're too verbose,
        // and with them the profile file quickly grows to several GBs of data.
        let profile_layer = profile_layer.with_filter(
            Targets::new()
                .with_default(LevelFilter::TRACE)
                .with_target("salsa", LevelFilter::WARN)
                .with_target("pubgrub", LevelFilter::WARN),
        );

        guard = Some(profile_layer_guard);
        Some(profile_layer)
    } else {
        None
    };

    tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(fmt_layer)
            .with(profile_layer),
    )
    .expect("could not set up global logger");

    guard
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
        .profile(args.profile_spec.try_into()?)
        .load_proc_macros(!args.no_proc_macros)
        .load_prebuilt_proc_macros(!args.no_prebuilt_proc_macros)
        .build()?;

    commands::run(args.command, &mut config)
}
