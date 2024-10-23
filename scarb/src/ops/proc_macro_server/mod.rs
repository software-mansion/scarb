use crate::compiler::plugin::proc_macro::ProcMacroHost;
use anyhow::{anyhow, Result};
use connection::Connection;
use json_rpc::ErrResponse;
use proc_macro_server_api::RpcResponse;

mod connection;
mod json_rpc;

pub fn start_proc_macro_server(proc_macros: ProcMacroHost) -> Result<()> {
    let connection = Connection::new();

    for i in 0..4 {
        let receiver = connection.receiver.clone();
        let sender = connection.sender.clone();

        std::thread::Builder::new()
            .name(format!("proc-macro-server-worker-thread-{i}"))
            .spawn(move || {
                for request in receiver {
                    let response = match request.method.as_str() {
                        //TODO add method handlers
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
