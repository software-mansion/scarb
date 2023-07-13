use anyhow::Result;
use clap::{Parser, Subcommand};

mod create_archive;
mod list_binaries;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    CreateArchive(create_archive::Args),
    ListBinaries,
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Command::CreateArchive(args) => create_archive::main(args),
        Command::ListBinaries => list_binaries::main(),
    }
}
