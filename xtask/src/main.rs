use anyhow::Result;
use clap::{Parser, Subcommand};

mod create_archive;
mod list_binaries;
mod verify_archive;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    CreateArchive(create_archive::Args),
    ListBinaries,
    VerifyArchive(verify_archive::Args),
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Command::CreateArchive(args) => create_archive::main(args),
        Command::ListBinaries => list_binaries::main(),
        Command::VerifyArchive(args) => verify_archive::main(args),
    }
}
