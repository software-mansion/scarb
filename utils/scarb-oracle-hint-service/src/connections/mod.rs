use crate::hint_service::OracleHintService;

#[cfg(feature = "shell")]
mod shell;
#[cfg(feature = "stdio")]
mod stdio_jsonrpc;
#[cfg(feature = "wasm")]
mod wasm;

pub fn add_builtin_protocols(hint_service: &mut OracleHintService) {
    #[cfg(feature = "stdio")]
    hint_service.add_protocol::<stdio_jsonrpc::StdioJsonRpc>();
    #[cfg(feature = "shell")]
    hint_service.add_protocol::<shell::Shell>();
    #[cfg(feature = "wasm")]
    hint_service.add_protocol::<wasm::Wasm>();
}
