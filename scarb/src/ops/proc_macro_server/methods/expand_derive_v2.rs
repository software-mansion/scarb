use std::sync::Arc;

use anyhow::{Context, Result};
use cairo_lang_macro_v2::TokenStream;
use convert_case::{Case, Casing};
use scarb_proc_macro_server_types::methods::{expand::ExpandDerive, ProcMacroResult};

use super::Handler;
use crate::compiler::plugin::{
    collection::WorkspaceProcMacros,
    proc_macro_common::{Expansion, ExpansionKind},
};

impl Handler for ExpandDerive {
    fn handle(
        workspace_macros: Arc<WorkspaceProcMacros>,
        params: Self::Params,
    ) -> Result<Self::Response> {
        let Self::Params {
            context,
            derives,
            item,
            call_site,
        } = params;

        let mut derived_code = TokenStream::empty();
        let mut all_diagnostics = vec![];

        for derive in derives {
            let expansion = Expansion::new(derive.to_case(Case::Snake), ExpansionKind::Derive);

            let plugin = workspace_macros
                .get(&context.package_id)
                .with_context(|| format!("No macros found in scope {context:?}"))?;

            let instance = plugin
                .macros()
                .iter()
                .find(|instance| instance.get_expansions().contains(&expansion))
                .with_context(|| format!("Unsupported derive macro: {derive}"))?;

            let result = instance.plugin().as_v2().unwrap().generate_code(
                expansion.name.clone(),
                call_site.clone(),
                TokenStream::empty(),
                item.clone(),
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
