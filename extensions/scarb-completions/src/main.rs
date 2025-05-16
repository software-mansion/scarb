use clap::Parser;
use scarb_completions::args::Args;
use scarb_completions::main_inner;
use scarb_ui::{OutputFormat, Ui, Verbosity};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = Args::parse();
    let ui = Ui::new(Verbosity::Normal, OutputFormat::Text);

    match main_inner(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            ui.error(format!("{error:#}"));
            ExitCode::FAILURE
        }
    }
}
