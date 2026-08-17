//! A minimal TCP echo server, used to exercise network access from within wasm oracles.
//!
//! This exists so that the `network` fixture can be tested against a peer we control, instead of
//! a public echo service on the internet, which makes the test hermetic and immune to third-party
//! outages.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

/// Environment variable carrying the `host:port` address of this server.
///
/// Keep in sync with the constant of the same name in the `wasip2` fixture source.
pub const ECHO_SERVER_ADDRESS_ENV: &str = "SCARB_TEST_ECHO_SERVER_ADDRESS";

/// Starts an echo server on an ephemeral loopback port, serving connections in the background
/// for the remaining lifetime of the test process.
///
/// Returns the address to connect to. A host name is used rather than a bare IP address, so that
/// the guest has to go through WASI name resolution to reach it.
pub fn spawn() -> String {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();

    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            thread::spawn(move || echo(stream));
        }
    });

    format!("localhost:{port}")
}

fn echo(mut stream: TcpStream) {
    // Clients are expected to shut down their writing side when done sending, which shows up
    // here as end of stream.
    let mut message = Vec::new();
    if stream.read_to_end(&mut message).is_ok() {
        let _ = stream.write_all(&message);
        let _ = stream.flush();
    }
}
