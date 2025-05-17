use clap::Parser;
use scarb_cairo_test::args::Args;
use scarb_ui::{OutputFormat, Ui};

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let ui = Ui::new(args.verbose.clone().into(), OutputFormat::Text);
    scarb_cairo_test::main_inner(&ui, args)
}
