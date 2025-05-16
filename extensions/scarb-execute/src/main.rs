use clap::Parser;
use scarb_execute::main_inner;
use scarb_extensions_cli::execute::Args;
use scarb_ui::Ui;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = Args::parse();
    let ui = Ui::new(args.verbose.clone().into(), scarb_ui::OutputFormat::Text);

    match main_inner(args, ui.clone()) {
        Ok(_execution_id) => ExitCode::SUCCESS,
        Err(error) => {
            ui.error(format!("{error:#}"));
            ExitCode::FAILURE
        }
    }
}
