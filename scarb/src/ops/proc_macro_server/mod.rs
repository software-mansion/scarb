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
use scarb_proc_macro_server_types::methods::discover_workspace::DiscoverWorkspace;
use scarb_proc_macro_server_types::methods::expand::{ExpandAttribute, ExpandDerive, ExpandInline};
use serde_json::Value;
use tracing::error;

use crate::core::Config;
use crate::ops::proc_macro_server::methods::discover_workspace::discover_workspace;
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
    // After closing it, threads stop and don't use the reference anymore.
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
    config: &'static Config,
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
    config: &'static Config,
    proc_macros: Arc<Mutex<ProcMacroStore>>,
    request: RpcRequest,
) -> Result<Value> {
    let result = match request.method.as_str() {
        DiscoverWorkspace::METHOD => discover_workspace(
            config,
            proc_macros.clone(),
            serde_json::from_value(request.value).unwrap(),
        )
        .map(|res| serde_json::to_value(res).unwrap()),
        DefinedMacros::METHOD => run_handler::<DefinedMacros>(proc_macros.clone(), request.value),
        ExpandAttribute::METHOD => {
            run_handler::<ExpandAttribute>(proc_macros.clone(), request.value)
        }
        ExpandDerive::METHOD => run_handler::<ExpandDerive>(proc_macros.clone(), request.value),
        ExpandInline::METHOD => run_handler::<ExpandInline>(proc_macros.clone(), request.value),
        _ => Err(anyhow!("method not found")),
    };

    if let Err(err) = &result {
        error!("[PMS] Error in {}: {:?}", request.method.as_str(), err)
    };

    result
}

fn run_handler<M: Handler>(proc_macros: Arc<Mutex<ProcMacroStore>>, value: Value) -> Result<Value> {
    M::handle(proc_macros, serde_json::from_value(value).unwrap())
        .map(|res| serde_json::to_value(res).unwrap())
}
