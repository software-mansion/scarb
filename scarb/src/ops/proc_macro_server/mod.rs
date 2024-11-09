use std::num::NonZero;
use std::sync::Arc;
use std::thread::available_parallelism;

use anyhow::{anyhow, Result};
use connection::Connection;
use crossbeam_channel::{Receiver, Sender};
use methods::Handler;
use scarb_proc_macro_server_types::jsonrpc::{ResponseError, RpcRequest, RpcResponse};
use scarb_proc_macro_server_types::methods::defined_macros::DefinedMacros;
use scarb_proc_macro_server_types::methods::expand::{ExpandAttribute, ExpandDerive};
use scarb_proc_macro_server_types::methods::Method;
use serde_json::Value;

use crate::compiler::plugin::proc_macro::ProcMacroHost;

mod connection;
mod methods;

pub fn start_proc_macro_server(proc_macros: ProcMacroHost) -> Result<()> {
    let connection = Connection::new();
    let available_parallelism = available_parallelism().map(NonZero::get).unwrap_or(4);
    let proc_macros = Arc::new(proc_macros);

    for i in 0..available_parallelism {
        let receiver = connection.receiver.clone();
        let sender = connection.sender.clone();
        let proc_macros = proc_macros.clone();

        std::thread::Builder::new()
            .name(format!("proc-macro-server-worker-thread-{i}"))
            .spawn(move || {
                handle_requests(proc_macros, receiver, sender);
            })
            .expect("failed to spawn thread");
    }

    connection.join();

    Ok(())
}

fn handle_requests(
    proc_macros: Arc<ProcMacroHost>,
    receiver: Receiver<RpcRequest>,
    sender: Sender<RpcResponse>,
) {
    for request in receiver {
        let id = request.id;
        let response = route_request(proc_macros.clone(), request);

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

fn route_request(proc_macros: Arc<ProcMacroHost>, request: RpcRequest) -> Result<Value> {
    match request.method.as_str() {
        DefinedMacros::METHOD => run_handler::<DefinedMacros>(proc_macros.clone(), request.value),
        ExpandAttribute::METHOD => {
            run_handler::<ExpandAttribute>(proc_macros.clone(), request.value)
        }
        ExpandDerive::METHOD => run_handler::<ExpandDerive>(proc_macros.clone(), request.value),
        _ => Err(anyhow!("method not found")),
    }
}

fn run_handler<M: Handler>(proc_macros: Arc<ProcMacroHost>, value: Value) -> Result<Value> {
    M::handle(proc_macros, serde_json::from_value(value).unwrap())
        .map(|res| serde_json::to_value(res).unwrap())
}
