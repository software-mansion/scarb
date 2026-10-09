//! Conversions between the two procedural macro api versions.

use cairo_lang_macro::{
    Diagnostic as DiagnosticV2, Severity as SeverityV2, TextSpan, TokenStream as TokenStreamV2,
    TokenTree,
};
use cairo_lang_macro_v1::{
    Diagnostic as DiagnosticV1, Severity as SeverityV1, TokenStream as TokenStreamV1,
    TokenStreamMetadata as TokenStreamMetadataV1,
};
use scarb_proc_macro_server_types::methods::SpannedTokenStream;

/// Downcasts the spanned token stream to the flat v1 one.
pub fn token_stream_v2_to_v1(token_stream_v2: &TokenStreamV2) -> TokenStreamV1 {
    let metadata_v2 = token_stream_v2.metadata.clone();
    let token_stream = TokenStreamV1::new(token_stream_v2.to_string());
    token_stream.with_metadata(TokenStreamMetadataV1 {
        original_file_path: metadata_v2.original_file_path,
        file_id: metadata_v2.file_id,
    })
}

/// Upcasts a flat v1 token stream to a single token spanning `origin`.
pub fn token_stream_v1_to_spanned(
    token_stream_v1: &TokenStreamV1,
    origin: TextSpan,
) -> SpannedTokenStream {
    SpannedTokenStream::unspanned(token_stream_v1.to_string(), origin)
}

/// The span covering a whole token stream, or `None` if it carries no tokens.
pub fn token_stream_span(token_stream: &TokenStreamV2) -> Option<TextSpan> {
    let TokenTree::Ident(first) = token_stream.tokens.first()?;
    let TokenTree::Ident(last) = token_stream.tokens.last()?;
    Some(TextSpan::new(first.span.start, last.span.end))
}

/// Upcasts the old diagnostic struct to the new one.
pub fn diagnostic_v1_to_v2(diagnostic_v1: &DiagnosticV1) -> DiagnosticV2 {
    DiagnosticV2::new(
        match diagnostic_v1.severity {
            SeverityV1::Error => SeverityV2::Error,
            SeverityV1::Warning => SeverityV2::Warning,
        },
        diagnostic_v1.message.clone(),
    )
}
