use clap::Parser;
use scarb_execute::main_inner;
use scarb_extensions_cli::execute::Args;
use scarb_ui::Ui;
use scarb_ui::args::VerbositySpec;
use std::process::ExitCode;
use std::str::FromStr;

fn main() -> ExitCode {
    let args = Args::parse();
    let ui = Ui::new(args.verbose.clone().into(), scarb_ui::OutputFormat::Text);

    init_logging(args.verbose.clone(), &ui);

    match main_inner(args, ui.clone()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            ui.error(format!("{error:#}"));
            ExitCode::FAILURE
        }
    }
}

fn init_logging(verbose: VerbositySpec, ui: &Ui) {
    use tracing_subscriber::filter::EnvFilter;
    use tracing_subscriber::fmt::time::Uptime;
    use tracing_subscriber::prelude::*;

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(ui.has_colors_enabled_stderr())
        .with_timer(Uptime::default())
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(FromStr::from_str(verbose.as_trace().as_str()).unwrap())
                .with_env_var("SCARB_LOG")
                .from_env_lossy(),
        )
        .init();
}
