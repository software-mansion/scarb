use std::sync::Arc;

use anyhow::Result;
use cairo_lang_macro::TokenStream;
use scarb_proc_macro_server_types::{
    context::RequestContext,
    methods::{expand::ExpandInline, ProcMacroResult},
};

use super::Handler;
use crate::compiler::plugin::{collection::WorkspaceProcMacros, proc_macro::ExpansionKind};

impl Handler for ExpandInline {
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
            name,
            args,
        } = params;

        let plugin = workspace_macros.get(&compilation_unit_id, &compilation_unit_component_id)?;

        let instance = plugin
            .macros
            .iter()
            .find(|instance| {
                instance
                    .get_expansions()
                    .iter()
                    .filter(|expansion| expansion.kind == ExpansionKind::Inline)
                    .any(|expansion| expansion.name == name)
            })
            .unwrap();

        let result = instance.generate_code(name.into(), TokenStream::empty(), args);

        Ok(ProcMacroResult {
            token_stream: result.token_stream,
            diagnostics: result.diagnostics,
        })
    }
}
