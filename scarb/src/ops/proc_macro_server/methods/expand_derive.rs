use crate::{
    compiler::plugin::proc_macro::{Expansion, ExpansionKind, ProcMacroHost},
    ops::proc_macro_server::json_rpc::Handler,
};
use anyhow::Result;
use cairo_lang_macro::TokenStream;
use convert_case::{Case, Casing};
use proc_macro_server_api::methods::{expand::ExpandDerive, ProcMacroResult};
use std::sync::Arc;

impl Handler for ExpandDerive {
    fn handle(proc_macros: Arc<ProcMacroHost>, params: Self::Params) -> Result<Self::Response> {
        let mut derived_code = String::new();
        let mut all_diagnostics = vec![];

        for derive in params.derives {
            let expansion = Expansion::new(derive.to_case(Case::Snake), ExpansionKind::Derive);
            let package_id = proc_macros
                .macros()
                .into_iter()
                .find(|e| e.get_expansions().contains(&expansion))
                .map(|m| m.package_id())
                .unwrap();

            let instance = proc_macros
                .macros()
                .iter()
                .find(|m| m.package_id() == package_id)
                .unwrap();

            let result = instance.generate_code(
                expansion.name.clone(),
                TokenStream::empty(),
                params.item.clone(),
            );

            // Register diagnostics.
            all_diagnostics.extend(result.diagnostics);

            if result.token_stream.is_empty() {
                // No code has been generated.
                // We do not need to do anything.
                continue;
            }

            derived_code.push_str(result.token_stream.to_string().as_str());
        }

        Ok(ProcMacroResult {
            token_stream: TokenStream::new(derived_code),
            diagnostics: all_diagnostics,
        })
    }
}
