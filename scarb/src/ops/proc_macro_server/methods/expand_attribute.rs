use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};

use cairo_lang_macro::{TextSpan, TokenStream as TokenStreamV2, TokenTree};
use cairo_lang_macro_v1::TokenStream as TokenStreamV1;
use scarb_proc_macro_server_types::methods::{
    ProcMacroResult, SpannedTokenStream, expand::ExpandAttribute,
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
                span_of(&item).unwrap_or(adapted_call_site),
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

/// A v1 macro reports no spans, so its expansion is attributed to `item_span`.
fn expand_attribute_v1(
    proc_macro_instance: &Arc<ProcMacroInstance>,
    fingerprint: u64,
    attr: String,
    item_span: TextSpan,
    args: TokenStreamV1,
    item: TokenStreamV1,
) -> Result<ProcMacroResult> {
    let result = proc_macro_instance
        .try_v1()?
        .generate_code(attr.into(), args, item);

    Ok(ProcMacroResult {
        token_stream: token_stream_v1_to_spanned(&result.token_stream, item_span),
        diagnostics: result.diagnostics.iter().map(diagnostic_v1_to_v2).collect(),
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

/// The span covering all tokens of `token_stream`, if it has any.
fn span_of(token_stream: &TokenStreamV2) -> Option<TextSpan> {
    let spans = || {
        token_stream
            .tokens
            .iter()
            .map(|TokenTree::Ident(token)| &token.span)
    };
    Some(TextSpan {
        start: spans().map(|span| span.start).min()?,
        end: spans().map(|span| span.end).max()?,
    })
}
