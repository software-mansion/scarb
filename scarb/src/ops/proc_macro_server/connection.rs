use crossbeam_channel::{Receiver, Sender};
use proc_macro_server_api::{RpcRequest, RpcResponse};
use std::{
    io::{BufRead, Write},
    thread::JoinHandle,
};

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

                if stdin.read_line(&mut line).is_err() {
                    eprintln!("Error occurred while reading from stdin");

                    break;
                }

                if line.is_empty() {
                    continue;
                }

                let Ok(request) = serde_json::from_str(&line) else {
                    eprintln!("Error occurred while deserializing request, used input:\n{line}");

                    break;
                };

                if reader_sender.send(request).is_err() {
                    eprintln!("Error occurred while sending request to worker threads");

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
                    eprintln!("Error occurred while writing to stdout");

                    break;
                }

                if stdout.flush().is_err() {
                    eprintln!("Error occurred while flushing stdout");

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
