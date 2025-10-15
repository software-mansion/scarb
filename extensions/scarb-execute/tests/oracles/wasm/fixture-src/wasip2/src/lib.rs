use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};

wit_bindgen::generate!({
    inline: r#"
        package testing:oracle;

        world oracle {
            export add: func(left: u64, right: u64) -> u64;
            export join: func(a: string, b: string) -> string;
            export io: func();
            export count: func() -> u64;
            export fs: func() -> result<string, string>;
            export network: func() -> result<string, string>;
        }
    "#
});

struct MyOracle;

impl Guest for MyOracle {
    fn add(left: u64, right: u64) -> u64 {
        left + right
    }
    fn join(a: String, b: String) -> String {
        a + &b
    }
    fn io() {
        println!("stdout is working as expected");
        eprintln!("stderr is working as expected");
    }
    fn count() -> u64 {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }
    fn fs() -> Result<String, String> {
        fs::write("write_file.txt", "hello from wasm").map_err(|e| e.to_string())?;
        fs::read_to_string("read_file.txt").map_err(|e| e.to_string())
    }
    fn network() -> Result<String, String> {
        let mut stream = TcpStream::connect("tcpbin.com:4242").map_err(|e| e.to_string())?;
        let message = "Hello World!";
        stream.write_all(message.as_bytes()).map_err(|e| e.to_string())?;
        stream.flush().map_err(|e| e.to_string())?;
        
        // Shut down the writing side to signal we're done sending
        stream.shutdown(std::net::Shutdown::Write).map_err(|e| e.to_string())?;
        
        let mut buffer = vec![0u8; message.len()];
        stream.read_exact(&mut buffer).map_err(|e| e.to_string())?;
        
        String::from_utf8(buffer).map_err(|e| e.to_string())
    }
}

export!(MyOracle);
