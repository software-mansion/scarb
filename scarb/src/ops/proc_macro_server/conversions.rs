//! Conversions between the two procedural macro api versions.
//!
//! The proc macro server speaks a single, v2-shaped protocol. Expansions performed through the v1
//! api, which knows nothing about token spans, are downgraded on the way in and upcast on the way
//! out, so that callers never have to care which api version a macro was built against.

use cairo_lang_macro::{
    Diagnostic as DiagnosticV2, Severity as SeverityV2, TextSpan, TokenStream as TokenStreamV2,
    TokenTree,
};
use scarb_proc_macro_server_types::methods::SpannedTokenStream;
use cairo_lang_macro_v1::{
    Diagnostic as DiagnosticV1, Severity as SeverityV1, TokenStream as TokenStreamV1,
    TokenStreamMetadata as TokenStreamMetadataV1,
};

/// Downcasts the spanned token stream to the flat v1 one.
pub fn token_stream_v2_to_v1(token_stream_v2: &TokenStreamV2) -> TokenStreamV1 {
    let metadata_v2 = token_stream_v2.metadata.clone();
    let token_stream = TokenStreamV1::new(token_stream_v2.to_string());
    token_stream.with_metadata(TokenStreamMetadataV1 {
        original_file_path: metadata_v2.original_file_path,
        file_id: metadata_v2.file_id,
    })
}

/// Upcasts a flat v1 token stream to a spanned one.
///
/// A v1 macro reports no span information whatsoever, so the best that can be done is to attribute
/// the whole expansion output to a single span of the original code: the item the macro was
/// applied to for attributes, and the macro call itself for derives and inline macros. Callers
/// then map the entire expansion back onto that span, rather than token by token.
pub fn token_stream_v1_to_spanned(
    token_stream_v1: &TokenStreamV1,
    origin: TextSpan,
) -> SpannedTokenStream {
    // An empty expansion means "remove the original item", which `single` preserves.
    SpannedTokenStream::unspanned(token_stream_v1.to_string(), origin)
}

/// The span covering a whole token stream, or `None` if it carries no tokens.
pub fn token_stream_span(token_stream: &TokenStreamV2) -> Option<TextSpan> {
    let TokenTree::Ident(first) = token_stream.tokens.first()?;
    let TokenTree::Ident(last) = token_stream.tokens.last()?;
    Some(TextSpan::new(first.span.start, last.span.end))
}

/// Upcasts the old diagnostic struct to the new one.
///
/// Note that v1 diagnostics carry no span, so the resulting diagnostic is reported at the call
/// site of the macro that produced it.
pub fn diagnostic_v1_to_v2(diagnostic_v1: &DiagnosticV1) -> DiagnosticV2 {
    DiagnosticV2::new(
        match diagnostic_v1.severity {
            SeverityV1::Error => SeverityV2::Error,
            SeverityV1::Warning => SeverityV2::Warning,
        },
        diagnostic_v1.message.clone(),
    )
}
