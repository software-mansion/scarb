use clap::Parser;
use scarb_cairo_run::args::Args;
use scarb_ui::{OutputFormat, Ui};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = Args::parse();
    let ui = Ui::new(args.verbose.clone().into(), OutputFormat::Text);
    ui.warn("`scarb cairo-run` will be deprecated soon\nhelp: use `scarb execute` instead");
    match scarb_cairo_run::main_inner(&ui, args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            ui.anyhow(&err);
            ExitCode::FAILURE
        }
    }
}
