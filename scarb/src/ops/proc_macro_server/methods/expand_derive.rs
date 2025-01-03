use std::sync::Arc;

use anyhow::Result;
use cairo_lang_macro::TokenStream;
use convert_case::{Case, Casing};
use scarb_proc_macro_server_types::methods::{expand::ExpandDerive, ProcMacroResult};

use super::Handler;
use crate::compiler::plugin::proc_macro::{Expansion, ExpansionKind, ProcMacroHost};

impl Handler for ExpandDerive {
    fn handle(proc_macro_host: Arc<ProcMacroHost>, params: Self::Params) -> Result<Self::Response> {
        let mut derived_code = TokenStream::empty();
        let mut all_diagnostics = vec![];

        for derive in params.derives {
            let expansion = Expansion::new(derive.to_case(Case::Snake), ExpansionKind::Derive);
            let instance = proc_macro_host
                .macros()
                .iter()
                .find(|e| e.get_expansions().contains(&expansion))
                .unwrap();

            let result = instance.generate_code(
                expansion.name.clone(),
                params.call_site.clone(),
                TokenStream::empty(),
                params.item.clone(),
            );

            // Register diagnostics.
            all_diagnostics.extend(result.diagnostics);
            // Add generated code.
            derived_code.tokens.extend(result.token_stream.tokens);
        }

        Ok(ProcMacroResult {
            token_stream: derived_code,
            diagnostics: all_diagnostics,
        })
    }
}
