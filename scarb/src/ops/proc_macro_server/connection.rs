use std::io::{BufRead, Write};
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender};
use scarb_proc_macro_server_types::jsonrpc::{RpcRequest, RpcResponse};
use tracing::error;

pub struct Connection {
    pub sender: Sender<RpcResponse>,
    pub receiver: Receiver<RpcRequest>,
    io_threads: IoThreads,
}

impl Connection {
    pub fn new() -> Self {
        let (reader_sender, reader_receiver) = crossbeam_channel::bounded(0);
        let (writer_sender, writer_receiver) = crossbeam_channel::bounded(0);

        let reader = std::thread::spawn(move || {
            let stdin = std::io::stdin();
            let mut stdin = stdin.lock();
            let mut line = String::new();

            loop {
                line.clear();

                match stdin.read_line(&mut line) {
                    // End of stream.
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(_) => {
                        // Report unexpected error.
                        error!("Reading from stdin failed");
                        break;
                    }
                };

                if line.is_empty() {
                    continue;
                }

                let Ok(request) = serde_json::from_str(&line) else {
                    error!("Deserializing request failed, used input:\n{line}");
                    break;
                };

                if reader_sender.send(request).is_err() {
                    break;
                }
            }
        });

        let writer = std::thread::spawn(move || {
            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();

            for response in writer_receiver {
                // This should not fail.
                let mut res = serde_json::to_vec(&response).unwrap();

                res.push(b'\n');

                if stdout.write_all(&res).is_err() {
                    error!("Writing to stdout failed");
                    break;
                }

                if stdout.flush().is_err() {
                    error!("Flushing stdout failed");
                    break;
                }
            }
        });

        let io_threads = IoThreads { reader, writer };

        Self {
            sender: writer_sender,
            receiver: reader_receiver,
            io_threads,
        }
    }

    pub fn join(self) {
        // There are clones of these used by worker threads. Drop only our refs.
        drop(self.sender);
        drop(self.receiver);

        if let Err(err) = self.io_threads.reader.join() {
            std::panic::panic_any(err);
        }
        if let Err(err) = self.io_threads.writer.join() {
            std::panic::panic_any(err);
        }
    }
}

struct IoThreads {
    reader: JoinHandle<()>,
    writer: JoinHandle<()>,
}
