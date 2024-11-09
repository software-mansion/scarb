use std::sync::Arc;

use anyhow::Result;
use cairo_lang_macro::TokenStream;
use scarb_proc_macro_server_types::methods::{expand::ExpandInline, ProcMacroResult};

use super::Handler;
use crate::compiler::plugin::proc_macro::{ExpansionKind, ProcMacroHost};

impl Handler for ExpandInline {
    fn handle(proc_macro_host: Arc<ProcMacroHost>, params: Self::Params) -> Result<Self::Response> {
        let instance = proc_macro_host
            .macros()
            .iter()
            .find(|e| {
                e.get_expansions()
                    .iter()
                    .filter(|expansion| expansion.kind == ExpansionKind::Inline)
                    .any(|expansion| expansion.name == params.name)
            })
            .unwrap();

        let result = instance.generate_code(params.name.into(), TokenStream::empty(), params.args);

        Ok(ProcMacroResult {
            token_stream: result.token_stream,
            diagnostics: result.diagnostics,
        })
    }
}
