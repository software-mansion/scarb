use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};

use cairo_lang_macro::{TextSpan, TokenStream};
use scarb_proc_macro_server_types::methods::{
    ProcMacroResult, SpannedTokenStream, expand::ExpandAttribute,
};

use super::Handler;
use crate::compiler::plugin::proc_macro::{
    ExpansionKind, ExpansionQuery, ProcMacroApiVersion, ProcMacroInstance,
};
use crate::core::Config;
use crate::ops::proc_macro_server::conversions::{
    diagnostic_v1_to_v2, token_stream_span, token_stream_v1_to_spanned, token_stream_v2_to_v1,
};
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
            ProcMacroApiVersion::V1 => {
                expand_attribute_v1(&proc_macro_instance, hash, attr, args, item)
            }
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
    args: TokenStream,
    item: TokenStream,
) -> Result<ProcMacroResult> {
    // A v1 macro sees the item as flat text, so the whole expansion is attributed back to the
    // whole item it was applied to.
    let origin = token_stream_span(&item).unwrap_or_else(|| TextSpan::new(0, 0));
    let result = proc_macro_instance.try_v1()?.generate_code(
        attr.into(),
        token_stream_v2_to_v1(&args),
        token_stream_v2_to_v1(&item),
    );

    Ok(ProcMacroResult {
        token_stream: token_stream_v1_to_spanned(&result.token_stream, origin),
        diagnostics: result.diagnostics.iter().map(diagnostic_v1_to_v2).collect(),
        fingerprint,
    })
}

fn expand_attribute_v2(
    proc_macro_instance: &Arc<ProcMacroInstance>,
    fingerprint: u64,
    attr: String,
    adapted_call_site: TextSpan,
    args: TokenStream,
    item: TokenStream,
) -> Result<ProcMacroResult> {
    let result =
        proc_macro_instance
            .try_v2()?
            .generate_code(attr.into(), adapted_call_site, args, item);

    Ok(ProcMacroResult {
        token_stream: SpannedTokenStream::from_token_stream(&result.token_stream),
        diagnostics: result.diagnostics,
        fingerprint,
    })
}
