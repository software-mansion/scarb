use std::io::Read;
use std::net::TcpStream;
use std::{env, io};

use anyhow::Result;
use clap::{Parser, Subcommand};
use scarb_test_support::simple_http_server::SimpleHttpServer;

#[derive(Parser, Clone, Debug)]
struct Args {
    /// Subcommand and its arguments.
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    HangOnTcp(HangOnTcpArgs),
    HttpServer,
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
        Command::HttpServer => http_server(),
    }
}

fn hang_on_tcp(args: HangOnTcpArgs) -> Result<()> {
    let address: &str = args.address.as_ref();

    let mut socket = TcpStream::connect(address).unwrap();
    let _ = socket.read(&mut [0; 10]);
    unreachable!("that read should never return");
}

#[tokio::main]
async fn http_server() -> Result<()> {
    let http = SimpleHttpServer::serve(env::current_dir().unwrap(), None);
    http.print_logs(true);
    println!("🚀 {}", http.url());
    println!("Press enter to continue...");
    let _ = io::stdin().read(&mut [0u8]).unwrap();
    drop(http);
    Ok(())
}
