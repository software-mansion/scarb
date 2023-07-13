use anyhow::Result;
use clap::{Parser, Subcommand};

mod list_binaries;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    ListBinaries,
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Command::ListBinaries => list_binaries::main(),
    }
}
