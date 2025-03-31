use std::sync::Arc;

use anyhow::{Context, Result};
use cairo_lang_macro_v1::TokenStream;
use convert_case::{Case, Casing};
use scarb_proc_macro_server_types::methods::{ProcMacroResult, expand::ExpandDerive};

use super::Handler;
use crate::compiler::plugin::proc_macro::{DeclaredProcMacroInstances, ProcMacroApiVersion};
use crate::compiler::plugin::{
    collection::WorkspaceProcMacros,
    proc_macro::{Expansion, ExpansionKind},
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
        } = params;

        let mut derived_code = String::new();
        let mut all_diagnostics = vec![];

        for derive in derives {
            let expansion = Expansion::new(derive.to_case(Case::Snake), ExpansionKind::Derive);

            let plugin = workspace_macros.get(&context.component);
            let plugin = plugin
                .as_ref()
                .and_then(|v| {
                    v.iter()
                        .find(|a| a.api_version() == ProcMacroApiVersion::V1)
                })
                .with_context(|| format!("No macros found in scope {context:?}"))?;

            let instance = plugin
                .instances()
                .iter()
                .find(|instance| instance.get_expansions().contains(&expansion))
                .with_context(|| format!("Unsupported derive macro: {derive}"))?;

            let result = instance
                .try_v1()
                .expect("procedural macro using v2 api used in a context expecting v1 api")
                .generate_code(expansion.name.clone(), TokenStream::empty(), item.clone());

            // Register diagnostics.
            all_diagnostics.extend(result.diagnostics);
            // Add generated code.
            derived_code.push_str(&result.token_stream.to_string());
        }

        Ok(ProcMacroResult {
            token_stream: TokenStream::new(derived_code),
            diagnostics: all_diagnostics,
        })
    }
}
