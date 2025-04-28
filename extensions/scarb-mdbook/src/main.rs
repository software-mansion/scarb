use clap::Parser;
use scarb_mdbook::args::Args;
use scarb_ui::{OutputFormat, Ui};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = Args::parse();
    let ui = Ui::new(args.verbose.clone().into(), OutputFormat::Text);
    let build_result = scarb_mdbook::main_inner(&args, ui.clone()).map_err(|err| {
        format!(
            "failed to build book from `{}` source path: {err:#}",
            args.input.clone()
        )
    });
    match build_result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            ui.error(error);
            ExitCode::FAILURE
        }
    }
}
