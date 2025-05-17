use clap::Parser;
use scarb_doc::args::Args;
use scarb_ui::{OutputFormat, Ui};

fn main() -> std::process::ExitCode {
    let args = Args::parse();
    let ui = Ui::new(args.verbose.clone().into(), OutputFormat::Text);
    match scarb_doc::main_inner(args, ui.clone()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            ui.error(format!("{error:#}"));
            std::process::ExitCode::FAILURE
        }
    }
}
