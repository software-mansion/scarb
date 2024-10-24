use crate::{
    compiler::plugin::proc_macro::{ExpansionKind, ProcMacroHost},
    ops::proc_macro_server::json_rpc::Handler,
};
use anyhow::Result;
use cairo_lang_macro::TokenStream;
use proc_macro_server_api::methods::expand::ExpandInline;
use std::sync::Arc;

impl Handler for ExpandInline {
    fn handle(proc_macros: Arc<ProcMacroHost>, params: Self::Params) -> Result<Self::Response> {
        let instance = proc_macros
            .macros()
            .into_iter()
            .find(|e| {
                e.get_expansions()
                    .iter()
                    .filter(|expansion| expansion.kind == ExpansionKind::Inline)
                    .any(|expansion| expansion.name == params.name)
            })
            .unwrap();

        let result = instance.generate_code(params.name.into(), TokenStream::empty(), params.item);

        Ok(proc_macro_server_api::methods::ProcMacroResult {
            token_stream: result.token_stream,
            diagnostics: result.diagnostics,
        })
    }
}
