use std::sync::Arc;

use anyhow::Result;
use scarb_proc_macro_server_types::methods::{expand::ExpandAttribute, ProcMacroResult};

use super::Handler;
use crate::compiler::plugin::proc_macro::{ExpansionKind, ProcMacroHost};

impl Handler for ExpandAttribute {
    fn handle(proc_macro_host: Arc<ProcMacroHost>, params: Self::Params) -> Result<Self::Response> {
        let instance = proc_macro_host
            .macros()
            .iter()
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
