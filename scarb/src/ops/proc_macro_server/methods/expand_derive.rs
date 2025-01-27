use std::sync::Arc;

use anyhow::Result;
use cairo_lang_macro::TokenStream;
use convert_case::{Case, Casing};
use scarb_proc_macro_server_types::{
    context::RequestContext,
    methods::{expand::ExpandDerive, ProcMacroResult},
};

use super::Handler;
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
            context:
                RequestContext {
                    compilation_unit_id,
                    compilation_unit_component_id,
                },
            derives,
            item,
        } = params;

        let mut derived_code = String::new();
        let mut all_diagnostics = vec![];

        for derive in derives {
            let expansion = Expansion::new(derive.to_case(Case::Snake), ExpansionKind::Derive);

            let plugin =
                workspace_macros.get(&compilation_unit_id, &compilation_unit_component_id)?;

            let instance = plugin
                .macros
                .iter()
                .find(|instance| instance.get_expansions().contains(&expansion))
                .unwrap();

            let result =
                instance.generate_code(expansion.name.clone(), TokenStream::empty(), item.clone());

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
