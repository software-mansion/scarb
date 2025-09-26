use std::mem;
use std::num::NonZero;
use std::sync::{Arc, Mutex};
use std::thread::available_parallelism;

use anyhow::{Result, anyhow};
use connection::Connection;
use crossbeam_channel::{Receiver, Sender};
use methods::Handler;
use scarb_proc_macro_server_types::jsonrpc::{ResponseError, RpcRequest, RpcResponse};
use scarb_proc_macro_server_types::methods::Method;
use scarb_proc_macro_server_types::methods::defined_macros::DefinedMacros;
use scarb_proc_macro_server_types::methods::expand::{ExpandAttribute, ExpandDerive, ExpandInline};
use serde_json::Value;

use crate::core::Config;
use crate::ops::store::ProcMacroStore;

mod connection;
mod methods;
pub mod store;

pub fn start_proc_macro_server(config: &Config) -> Result<()> {
    let connection = Connection::new();
    let available_parallelism = available_parallelism().map(NonZero::get).unwrap_or(4);

    // SAFETY:
    // Config can be transmuted to a static reference and propagated between threads
    // because all of those threads read from the connection receiver.
    // The connection lifetime is limited to the scope of this function.
    // After closing it, threads stop and don't use the reference anymore
    // so there is no risk of if any thread referencing the config
    // after it gets dropped somewhere outside this function.
    let config: &'static Config = unsafe { mem::transmute(config) };

    let proc_macro_store: Arc<Mutex<ProcMacroStore>> = Default::default();

    for i in 0..available_parallelism {
        let receiver = connection.receiver.clone();
        let sender = connection.sender.clone();
        let proc_macro_store = proc_macro_store.clone();

        std::thread::Builder::new()
            .name(format!("proc-macro-server-worker-thread-{i}"))
            .spawn(move || {
                handle_requests(config, proc_macro_store, receiver, sender);
            })
            .expect("failed to spawn thread");
    }

    connection.join();

    Ok(())
}

fn handle_requests(
    config: &Config,
    proc_macros: Arc<Mutex<ProcMacroStore>>,
    receiver: Receiver<RpcRequest>,
    sender: Sender<RpcResponse>,
) {
    for request in receiver {
        let id = request.id;
        let response = route_request(config, proc_macros.clone(), request);

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

fn route_request(
    config: &Config,
    proc_macros: Arc<Mutex<ProcMacroStore>>,
    request: RpcRequest,
) -> Result<Value> {
    let value = request.value;
    match request.method.as_str() {
        DefinedMacros::METHOD => run_handler::<DefinedMacros>(config, proc_macros, value),
        ExpandAttribute::METHOD => run_handler::<ExpandAttribute>(config, proc_macros, value),
        ExpandDerive::METHOD => run_handler::<ExpandDerive>(config, proc_macros, value),
        ExpandInline::METHOD => run_handler::<ExpandInline>(config, proc_macros, value),
        _ => Err(anyhow!("method not found")),
    }
}

fn run_handler<M: Handler>(
    config: &Config,
    proc_macros: Arc<Mutex<ProcMacroStore>>,
    value: Value,
) -> Result<Value> {
    M::handle(config, proc_macros, serde_json::from_value(value).unwrap())
        .map(|res| serde_json::to_value(res).unwrap())
}
