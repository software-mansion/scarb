use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};

use cairo_lang_macro::{TextSpan, TokenStream as TokenStreamV2};
use cairo_lang_macro_v1::TokenStream as TokenStreamV1;
use scarb_proc_macro_server_types::conversions::{diagnostic_v1_to_v2, token_stream_v2_to_v1};
use scarb_proc_macro_server_types::methods::{ProcMacroResult, expand::ExpandAttribute};

use super::{Handler, interface_code_mapping_from_cairo};
use crate::compiler::plugin::proc_macro::v2::generate_code_mappings;
use crate::compiler::plugin::proc_macro::{
    ExpansionKind, ExpansionQuery, ProcMacroApiVersion, ProcMacroInstance,
};
use crate::core::Config;
use crate::ops::store::ProcMacroStore;

impl Handler for ExpandAttribute {
    fn handle(
        _config: &Config,
        proc_macros: Arc<Mutex<ProcMacroStore>>,
        params: Self::Params,
    ) -> Result<Self::Response> {
        let Self::Params {
            context,
            attr,
            args,
            item,
            adapted_call_site,
        } = params;
        let expansion = ExpansionQuery::with_expansion_name(&attr, ExpansionKind::Attr);
        let (proc_macro_instance, hash) = proc_macros
            .lock()
            .unwrap()
            .get_instance_and_hash(&context, &expansion)
            .with_context(|| {
                format!("No \"{attr}\" attribute macros found in scope: {context:?}")
            })?;

        match proc_macro_instance.api_version() {
            ProcMacroApiVersion::V1 => expand_attribute_v1(
                &proc_macro_instance,
                hash,
                attr,
                token_stream_v2_to_v1(&args),
                token_stream_v2_to_v1(&item),
            ),
            ProcMacroApiVersion::V2 => expand_attribute_v2(
                &proc_macro_instance,
                hash,
                attr,
                adapted_call_site,
                args,
                item,
            ),
        }
    }
}

fn expand_attribute_v1(
    proc_macro_instance: &Arc<ProcMacroInstance>,
    fingerprint: u64,
    attr: String,
    args: TokenStreamV1,
    item: TokenStreamV1,
) -> Result<ProcMacroResult> {
    let result = proc_macro_instance
        .try_v1()?
        .generate_code(attr.into(), args, item);

    Ok(ProcMacroResult {
        token_stream: result.token_stream,
        diagnostics: result.diagnostics.iter().map(diagnostic_v1_to_v2).collect(),
        code_mappings: None,
        fingerprint,
    })
}

fn expand_attribute_v2(
    proc_macro_instance: &Arc<ProcMacroInstance>,
    fingerprint: u64,
    attr: String,
    adapted_call_site: TextSpan,
    args: TokenStreamV2,
    item: TokenStreamV2,
) -> Result<ProcMacroResult> {
    let result = proc_macro_instance.try_v2()?.generate_code(
        attr.into(),
        adapted_call_site.clone(),
        args,
        item,
    );

    let code_mappings = generate_code_mappings(&result.token_stream, adapted_call_site);
    Ok(ProcMacroResult {
        token_stream: token_stream_v2_to_v1(&result.token_stream),
        diagnostics: result.diagnostics,
        code_mappings: Some(
            code_mappings
                .into_iter()
                .map(interface_code_mapping_from_cairo)
                .collect(),
        ),
        fingerprint,
    })
}
