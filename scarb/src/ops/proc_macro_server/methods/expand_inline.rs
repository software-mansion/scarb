use std::sync::Arc;

use anyhow::{Context, Result};
use cairo_lang_macro::{TextSpan, TokenStream as TokenStreamV2};
use scarb_proc_macro_server_types::methods::{ProcMacroResult, expand::ExpandInline};

use super::{Handler, interface_code_mapping_from_cairo};
use crate::compiler::plugin::proc_macro::v2::generate_code_mappings;
use crate::compiler::plugin::proc_macro::{
    DeclaredProcMacroInstances, ExpansionQuery, ProcMacroApiVersion, ProcMacroInstance,
};
use crate::compiler::plugin::{collection::WorkspaceProcMacros, proc_macro::ExpansionKind};
use cairo_lang_macro_v1::TokenStream as TokenStreamV1;
use scarb_proc_macro_server_types::conversions::{diagnostic_v1_to_v2, token_stream_v2_to_v1};

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

        let expansion = ExpansionQuery::with_expansion_name(&name, ExpansionKind::Inline);
        let plugins = workspace_macros.get(&context.component);
        let proc_macro_instance = plugins
            .as_ref()
            .and_then(|v| {
                v.iter()
                    .filter_map(|plugin| plugin.find_instance_with_expansion(&expansion))
                    .next()
            })
            .with_context(|| format!("No \"{name}\" inline macros found in scope: {context:?}"))?;

        match proc_macro_instance.api_version() {
            ProcMacroApiVersion::V1 => {
                expand_inline_v1(proc_macro_instance, name, token_stream_v2_to_v1(&args))
            }
            ProcMacroApiVersion::V2 => expand_inline_v2(proc_macro_instance, name, call_site, args),
        }
    }
}

fn expand_inline_v1(
    proc_macro_instance: &Arc<ProcMacroInstance>,
    name: String,
    args: TokenStreamV1,
) -> Result<ProcMacroResult> {
    let result = proc_macro_instance.try_v1()?.generate_code(
        name.into(),
        TokenStreamV1::new("".to_string()),
        args.clone(),
    );

    Ok(ProcMacroResult {
        token_stream: result.token_stream,
        diagnostics: result.diagnostics.iter().map(diagnostic_v1_to_v2).collect(),
        code_mappings: None,
    })
}

fn expand_inline_v2(
    proc_macro_instance: &Arc<ProcMacroInstance>,
    name: String,
    call_site: TextSpan,
    args: TokenStreamV2,
) -> Result<ProcMacroResult> {
    let result = proc_macro_instance.try_v2()?.generate_code(
        name.into(),
        call_site.clone(),
        TokenStreamV2::empty(),
        args,
    );

    let code_mappings = generate_code_mappings(&result.token_stream, call_site);
    Ok(ProcMacroResult {
        token_stream: token_stream_v2_to_v1(&result.token_stream),
        diagnostics: result.diagnostics,
        code_mappings: Some(
            code_mappings
                .into_iter()
                .map(interface_code_mapping_from_cairo)
                .collect(),
        ),
    })
}
