use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use cairo_lang_macro::{TextSpan, TokenStream};
use scarb_proc_macro_server_types::methods::{
    ProcMacroResult, SpannedTokenStream, expand::ExpandDerive,
};

use super::Handler;
use crate::compiler::plugin::proc_macro::{
    Expansion, ExpansionKind, ExpansionQuery, ProcMacroApiVersion, ProcMacroInstance,
};
use crate::core::Config;
use crate::ops::proc_macro_server::conversions::{
    diagnostic_v1_to_v2, token_stream_v1_to_spanned, token_stream_v2_to_v1,
};
use crate::ops::store::ProcMacroStore;

impl Handler for ExpandDerive {
    fn handle(
        _config: &Config,
        proc_macros: Arc<Mutex<ProcMacroStore>>,
        params: Self::Params,
    ) -> Result<Self::Response> {
        let Self::Params {
            context,
            derive,
            item,
            call_site,
        } = params;

        let query = ExpansionQuery::with_expansion_name(derive.clone(), ExpansionKind::Derive);

        let (proc_macro_instance, hash) = proc_macros
            .lock()
            .unwrap()
            .get_instance_and_hash(&context, &query)
            .with_context(|| format!("No \"{derive}\" derive macros found in scope {context:?}"))?;

        let expansion = proc_macro_instance
            .find_expansion(&query)
            .with_context(|| format!("No \"{derive}\" derive macros found in scope {context:?}"))?
            .clone();

        match proc_macro_instance.api_version() {
            ProcMacroApiVersion::V1 => {
                expand_derive_v1(&proc_macro_instance, hash, &expansion, item, call_site)
            }
            ProcMacroApiVersion::V2 => {
                expand_derive_v2(&proc_macro_instance, hash, &expansion, call_site, item)
            }
        }
    }
}

fn expand_derive_v1(
    proc_macro_instance: &Arc<ProcMacroInstance>,
    fingerprint: u64,
    expansion: &Expansion,
    item: TokenStream,
    call_site: TextSpan,
) -> Result<ProcMacroResult> {
    let result = proc_macro_instance.try_v1()?.generate_code(
        expansion.expansion_name.clone(),
        cairo_lang_macro_v1::TokenStream::empty(),
        token_stream_v2_to_v1(&item),
    );

    // A v1 macro reports no spans, so the whole expansion is attributed to the derive call.
    Ok(ProcMacroResult {
        token_stream: token_stream_v1_to_spanned(&result.token_stream, call_site),
        diagnostics: result.diagnostics.iter().map(diagnostic_v1_to_v2).collect(),
        fingerprint,
    })
}

fn expand_derive_v2(
    proc_macro_instance: &Arc<ProcMacroInstance>,
    fingerprint: u64,
    expansion: &Expansion,
    call_site: TextSpan,
    item: TokenStream,
) -> Result<ProcMacroResult> {
    let result = proc_macro_instance.try_v2()?.generate_code(
        expansion.expansion_name.clone(),
        call_site,
        TokenStream::empty(),
        item,
    );

    Ok(ProcMacroResult {
        token_stream: SpannedTokenStream::from_token_stream(&result.token_stream),
        diagnostics: result.diagnostics,
        fingerprint,
    })
}
