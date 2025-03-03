use std::sync::Arc;

use anyhow::{Context, Result};
use cairo_lang_macro_v2::TokenStream;
use scarb_proc_macro_server_types::methods::{expand::ExpandInline, ProcMacroResult};

use super::Handler;
use crate::compiler::plugin::{collection::WorkspaceProcMacros, proc_macro_common::ExpansionKind};

impl Handler for ExpandInline {
    fn handle(
        workspace_macros: Arc<WorkspaceProcMacros>,
        params: Self::Params,
    ) -> Result<Self::Response> {
        let Self::Params {
            context,
            name,
            args,
            call_site,
        } = params;

        let plugin = workspace_macros
            .get(&context.package_id)
            .with_context(|| format!("No macros found in scope: {context:?}"))?;

        let instance = plugin
            .macros()
            .iter()
            .find(|instance| {
                instance
                    .get_expansions()
                    .iter()
                    .filter(|expansion| expansion.kind == ExpansionKind::Inline)
                    .any(|expansion| expansion.name == name)
            })
            .with_context(|| format!("Unsupported inline macro: {name}"))?;

        let result = instance.plugin().as_v2().unwrap().generate_code(
            name.into(),
            call_site,
            TokenStream::empty(),
            args,
        );

        Ok(ProcMacroResult {
            token_stream: result.token_stream,
            diagnostics: result.diagnostics,
        })
    }
}
