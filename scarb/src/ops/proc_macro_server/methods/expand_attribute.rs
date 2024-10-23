use crate::{
    compiler::plugin::proc_macro::{ExpansionKind, ProcMacroHost},
    ops::proc_macro_server::json_rpc::Handler,
};
use anyhow::Result;
use proc_macro_server_api::methods::{expand::ExpandAttribute, ProcMacroResult};
use std::sync::Arc;

impl Handler for ExpandAttribute {
    fn handle(proc_macros: Arc<ProcMacroHost>, params: Self::Params) -> Result<Self::Response> {
        let instance = proc_macros
            .macros()
            .into_iter()
            .find(|e| {
                e.get_expansions()
                    .iter()
                    .filter(|expansion| expansion.kind == ExpansionKind::Attr)
                    .any(|expansion| expansion.name == params.attr)
            })
            .unwrap();

        let result = instance.generate_code(params.attr.into(), params.args, params.item);

        Ok(ProcMacroResult {
            token_stream: result.token_stream,
            diagnostics: result.diagnostics,
        })
    }
}
