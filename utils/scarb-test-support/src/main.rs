use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser, Clone, Debug)]
struct Args {
    /// Subcommand and its arguments.
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    HangOnTcp(HangOnTcpArgs),
}

#[derive(Parser, Clone, Debug)]
pub struct HangOnTcpArgs {
    #[arg(short, long)]
    address: String,
}

fn main() -> Result<()> {
    let args: Args = Args::parse();
    match args.command {
        Command::HangOnTcp(args) => hang_on_tcp(args),
    }
}

fn hang_on_tcp(args: HangOnTcpArgs) -> Result<()> {
    use std::io::Read;
    use std::net::TcpStream;

    let address: &str = args.address.as_ref();

    let mut socket = TcpStream::connect(address).unwrap();
    let _ = socket.read(&mut [0; 10]);
    unreachable!("that read should never return");
}
