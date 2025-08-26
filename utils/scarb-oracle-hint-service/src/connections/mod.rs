use crate::hint_service::OracleHintService;

#[cfg(feature = "stdio")]
mod stdio_jsonrpc;

pub fn add_builtin_protocols(hint_service: &mut OracleHintService) {
    #[cfg(feature = "stdio")]
    hint_service.add_protocol::<stdio_jsonrpc::StdioJsonRpc>();
}
