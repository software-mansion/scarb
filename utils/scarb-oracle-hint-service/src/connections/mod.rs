use crate::hint_service::OracleHintService;

mod stdio_jsonrpc;

pub fn add_builtin_protocols(hint_service: &mut OracleHintService) {
    hint_service.add_protocol::<stdio_jsonrpc::StdioJsonRpc>();
}
