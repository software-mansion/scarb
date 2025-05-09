use clap::Parser;
use scarb_ui::{OutputFormat, Ui};
use scarb_verify::args::Args;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = Args::parse();
    let ui = Ui::new(args.verbose.clone().into(), OutputFormat::Text);

    match scarb_verify::main_inner(args, ui.clone()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            ui.error(format!("{error:#}"));
            ExitCode::FAILURE
        }
    }
}
