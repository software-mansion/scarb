use crate::compiler::plugin::proc_macro::ProcMacroHost;
use anyhow::{anyhow, Result};
use connection::Connection;
use json_rpc::{ErrResponse, Handler};
use proc_macro_server_api::{
    methods::{defined_macros::DefinedMacros, expand::ExpandAttribute},
    Method, RpcResponse,
};
use serde_json::Value;
use std::sync::Arc;

mod connection;
mod json_rpc;
mod methods;

pub fn start_proc_macro_server(proc_macros: ProcMacroHost) -> Result<()> {
    let connection = Connection::new();
    let proc_macros = Arc::new(proc_macros);

    for i in 0..4 {
        let receiver = connection.receiver.clone();
        let sender = connection.sender.clone();
        let proc_macros = proc_macros.clone();

        std::thread::Builder::new()
            .name(format!("proc-macro-server-worker-thread-{i}"))
            .spawn(move || {
                for request in receiver {
                    let response = match request.method.as_str() {
                        DefinedMacros::METHOD => {
                            run_handler::<DefinedMacros>(proc_macros.clone(), request.value)
                        }
                        ExpandAttribute::METHOD => {
                            run_handler::<ExpandAttribute>(proc_macros.clone(), request.value)
                        }
                        _ => Err(anyhow!("method not found")),
                    };

                    let value = response.unwrap_or_else(|err| {
                        serde_json::to_value(ErrResponse::new(err.to_string())).unwrap()
                    });
                    let res = RpcResponse {
                        id: request.id,
                        value,
                    };

                    sender.send(res).unwrap();
                }
            })
            .expect("failed to spawn thread");
    }

    connection.join();

    Ok(())
}

fn run_handler<M: Handler>(proc_macros: Arc<ProcMacroHost>, value: Value) -> Result<Value> {
    M::handle(proc_macros.clone(), serde_json::from_value(value).unwrap())
        .map(|res| serde_json::to_value(res).unwrap())
}
