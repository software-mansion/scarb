use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use cairo_lang_macro::{TextSpan, TokenStream};
use scarb_proc_macro_server_types::methods::{
    ProcMacroResult, SpannedTokenStream, expand::ExpandInline,
};

use super::Handler;
use crate::compiler::plugin::proc_macro::{
    ExpansionKind, ExpansionQuery, ProcMacroApiVersion, ProcMacroInstance,
};
use crate::core::Config;
use crate::ops::proc_macro_server::conversions::{
    diagnostic_v1_to_v2, token_stream_v1_to_spanned, token_stream_v2_to_v1,
};
use crate::ops::store::ProcMacroStore;

impl Handler for ExpandInline {
    fn handle(
        _config: &Config,
        proc_macros: Arc<Mutex<ProcMacroStore>>,
        params: Self::Params,
    ) -> Result<Self::Response> {
        let Self::Params {
            context,
            name,
            args,
            call_site,
        } = params;

        let expansion = ExpansionQuery::with_expansion_name(&name, ExpansionKind::Inline);
        let (proc_macro_instance, hash) = proc_macros
            .lock()
            .unwrap()
            .get_instance_and_hash(&context, &expansion)
            .with_context(|| format!("No \"{name}\" inline macros found in scope: {context:?}"))?;

        match proc_macro_instance.api_version() {
            ProcMacroApiVersion::V1 => {
                expand_inline_v1(&proc_macro_instance, hash, name, args, call_site)
            }
            ProcMacroApiVersion::V2 => {
                expand_inline_v2(&proc_macro_instance, hash, name, call_site, args)
            }
        }
    }
}

fn expand_inline_v1(
    proc_macro_instance: &Arc<ProcMacroInstance>,
    fingerprint: u64,
    name: String,
    args: TokenStream,
    call_site: TextSpan,
) -> Result<ProcMacroResult> {
    let result = proc_macro_instance.try_v1()?.generate_code(
        name.into(),
        cairo_lang_macro_v1::TokenStream::new(String::new()),
        token_stream_v2_to_v1(&args),
    );

    // A v1 macro reports no spans, so the whole expansion is attributed to the macro call.
    Ok(ProcMacroResult {
        token_stream: token_stream_v1_to_spanned(&result.token_stream, call_site),
        diagnostics: result.diagnostics.iter().map(diagnostic_v1_to_v2).collect(),
        fingerprint,
    })
}

fn expand_inline_v2(
    proc_macro_instance: &Arc<ProcMacroInstance>,
    fingerprint: u64,
    name: String,
    call_site: TextSpan,
    args: TokenStream,
) -> Result<ProcMacroResult> {
    let result = proc_macro_instance.try_v2()?.generate_code(
        name.into(),
        call_site,
        TokenStream::empty(),
        args,
    );

    Ok(ProcMacroResult {
        token_stream: SpannedTokenStream::from_token_stream(&result.token_stream),
        diagnostics: result.diagnostics,
        fingerprint,
    })
}
