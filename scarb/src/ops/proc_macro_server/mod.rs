use std::num::NonZero;
use std::thread::available_parallelism;

use anyhow::{anyhow, Result};
use connection::Connection;
use crossbeam_channel::{Receiver, Sender};
use scarb_proc_macro_server_types::jsonrpc::{ResponseError, RpcRequest, RpcResponse};
use serde_json::Value;

use crate::compiler::plugin::proc_macro::ProcMacroHost;

mod connection;

pub fn start_proc_macro_server(_proc_macros: ProcMacroHost) -> Result<()> {
    let connection = Connection::new();
    let available_parallelism = available_parallelism().map(NonZero::get).unwrap_or(4);

    for i in 0..available_parallelism {
        let receiver = connection.receiver.clone();
        let sender = connection.sender.clone();

        std::thread::Builder::new()
            .name(format!("proc-macro-server-worker-thread-{i}"))
            .spawn(move || {
                handle_requests(receiver, sender);
            })
            .expect("failed to spawn thread");
    }

    connection.join();

    Ok(())
}

fn handle_requests(receiver: Receiver<RpcRequest>, sender: Sender<RpcResponse>) {
    for request in receiver {
        let id = request.id;
        let response = route_request(request);

        let response = match response {
            Ok(result) => RpcResponse {
                id,
                result: Some(result),
                error: None,
            },
            Err(error) => RpcResponse {
                id,
                result: None,
                error: Some(ResponseError {
                    message: error.to_string(),
                }),
            },
        };

        sender.send(response).unwrap();
    }
}

fn route_request(request: RpcRequest) -> Result<Value> {
    #[allow(clippy::match_single_binding)]
    match request.method.as_str() {
        //TODO add method handlers
        _ => Err(anyhow!("method not found")),
    }
}
