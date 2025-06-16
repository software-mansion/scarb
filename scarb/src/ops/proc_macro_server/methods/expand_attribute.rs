use anyhow::{anyhow, Context, Result};
use std::sync::Arc;

use cairo_lang_macro_v1::TokenStream as TokenStreamV1;
use scarb_proc_macro_server_types::methods::{expand::ExpandAttribute, ProcMacroResult};

use super::Handler;
use crate::compiler::plugin::collection::WorkspaceProcMacros;
use crate::compiler::plugin::proc_macro::{
    DeclaredProcMacroInstances, ExpansionKind, ExpansionQuery, ProcMacroApiVersion,
    ProcMacroInstance,
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
        } = params;
        let expansion = ExpansionQuery::with_expansion_name(&attr, ExpansionKind::Attr);
        let plugins = workspace_macros.get(&context.component);
        let proc_macro_instance = plugins
            .as_ref()
            .and_then(|v| {
                v.iter()
                    .filter_map(|plugin| plugin.find_instance_with_expansion(&expansion))
                    .next()
            })
            .with_context(|| {
                format!("No \"{attr}\" attribute macros found in scope: {context:?}")
            })?;

        match proc_macro_instance.api_version() {
            ProcMacroApiVersion::V1 => expand_attribute_v1(proc_macro_instance, attr, args, item),
            ProcMacroApiVersion::V2 => Err(anyhow!("v2 used")),
        }
    }
}

fn expand_attribute_v1(
    proc_macro_instance: &Arc<ProcMacroInstance>,
    attr: String,
    args: TokenStreamV1,
    item: TokenStreamV1,
) -> Result<ProcMacroResult> {
    let result = proc_macro_instance
        .try_v1()?
        .generate_code(attr.into(), args, item);

    Ok(ProcMacroResult {
        token_stream: result.token_stream,
        diagnostics: result.diagnostics,
        code_mappings: None,
    })
}

// fn expand_attribute_v2(
//     proc_macro_instance: &Arc<ProcMacroInstance>,
//     attr: String,
//     call_site: TextSpan,
//     args: TokenStreamV2,
//     item: TokenStreamV2,
// ) -> Result<ProcMacroResult> {
//     let result =
//         proc_macro_instance
//             .try_v2()?
//             .generate_code(attr.into(), call_site.clone(), args, item);
//
//     let code_mappings = generate_code_mappings(&result.token_stream, call_site);
//     Ok(ProcMacroResult {
//         token_stream: token_stream_v2_to_v1(&result.token_stream),
//         diagnostics: result.diagnostics,
//         code_mappings: None,
//     })
// }
