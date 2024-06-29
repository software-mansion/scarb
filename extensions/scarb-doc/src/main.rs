use anyhow::Result;
use clap::Parser;
use scarb_ui::args::PackagesFilter;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(flatten)]
    packages_filter: PackagesFilter,
}

fn main() -> Result<()> {
    let _args = Args::parse();

    panic!("doc is not available in this build");
}
