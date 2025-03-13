use std::sync::Arc;

use anyhow::{Context, Result};
use scarb_proc_macro_server_types::methods::{ProcMacroResult, expand::ExpandAttribute};

use super::Handler;
use crate::compiler::plugin::collection::WorkspaceProcMacros;
use crate::compiler::plugin::proc_macro::{
    DeclaredProcMacroInstances, ExpansionKind, ProcMacroApiVersion,
};

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
            call_site,
        } = params;

        let plugin = workspace_macros.get(&context.component);
        let plugin = plugin
            .as_ref()
            .and_then(|v| {
                v.iter()
                    .find(|a| a.api_version() == ProcMacroApiVersion::V2)
            })
            .with_context(|| format!("No macros found in scope: {context:?}"))?;

        let instance = plugin
            .instances()
            .iter()
            .find(|instance| {
                instance
                    .get_expansions()
                    .iter()
                    .filter(|expansion| expansion.kind == ExpansionKind::Attr)
                    .any(|expansion| expansion.name == attr)
            })
            .with_context(|| format!("Unsupported attribute: {attr}"))?;

        let result = instance
            .try_v2()
            .expect("procedural macro using v1 api used in a context expecting v2 api")
            .generate_code(attr.into(), call_site, args, item);

        Ok(ProcMacroResult {
            token_stream: result.token_stream,
            diagnostics: result.diagnostics,
        })
    }
}
