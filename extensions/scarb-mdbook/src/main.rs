#![deny(clippy::dbg_macro)]
#![deny(clippy::disallowed_methods)]

use anyhow::{Context, Result};
use clap::Parser;
use mdbook_driver::MDBook;
use mimalloc::MiMalloc;
use scarb_extensions_cli::mdbook::Args;
use scarb_ui::Ui;
use scarb_ui::args::VerbositySpec;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> ExitCode {
    let args = Args::parse();
    let ui = Ui::new(args.verbose.clone().into(), scarb_ui::OutputFormat::Text);
    init_logging(args.verbose.clone(), &ui);
    let build_result = main_inner(&args, ui.clone()).with_context(|| {
        format!(
            "failed to build book from `{}` source path",
            args.input.clone()
        )
    });
    match build_result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            ui.error(format!("{error:#}"));
            ExitCode::FAILURE
        }
    }
}

fn main_inner(args: &Args, _ui: Ui) -> Result<()> {
    let mut book = MDBook::load(args.input.clone())?;
    let output_path: PathBuf = args.output.clone().into();
    book.config.build.build_dir = output_path
        .strip_prefix(&args.input)
        .unwrap_or(&output_path.clone())
        .into();
    book.build()?;
    let highlight = include_str!("../theme/highlight.js");
    let mut highlight_file = File::create(output_path.join("highlight.js"))?;
    highlight_file.write_all(highlight.as_bytes())?;
    Ok(())
}

fn init_logging(verbose: VerbositySpec, ui: &Ui) {
    use tracing_subscriber::filter::EnvFilter;
    use tracing_subscriber::fmt::time::Uptime;

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(ui.has_colors_enabled_stderr())
        .with_timer(Uptime::default())
        .with_env_filter(EnvFilter::new(
            verbose.as_trace().replace("scarb", "mdbook"),
        ))
        .init();
}
