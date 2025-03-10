use std::sync::Arc;

use anyhow::{Context, Result};
use scarb_proc_macro_server_types::methods::{ProcMacroResult, expand::ExpandAttribute};

use super::Handler;
use crate::compiler::plugin::{collection::WorkspaceProcMacros, proc_macro::ExpansionKind};

impl Handler for ExpandAttribute {
    fn handle(
        workspace_macros: Arc<WorkspaceProcMacros>,
        params: Self::Params,
    ) -> Result<Self::Response> {
        let Self::Params {
            context,
            attr,
            args,
            item,
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
                    .filter(|expansion| expansion.kind == ExpansionKind::Attr)
                    .any(|expansion| expansion.name == attr)
            })
            .with_context(|| format!("Unsupported attribute: {attr}"))?;

        let result = instance.generate_code(attr.into(), args, item);

        Ok(ProcMacroResult {
            token_stream: result.token_stream,
            diagnostics: result.diagnostics,
        })
    }
}
