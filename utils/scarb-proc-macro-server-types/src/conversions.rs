use cairo_lang_macro::TokenStream as TokenStreamV2;
use cairo_lang_macro_v1::{
    TokenStream as TokenStreamV1, TokenStreamMetadata as TokenStreamMetadataV1,
};
/// Downcasts new token stream to the old one
pub fn token_stream_v2_to_v1(token_stream_v2: &TokenStreamV2) -> TokenStreamV1 {
    let metadata_v2 = token_stream_v2.metadata.clone();
    let token_stream = TokenStreamV1::new(token_stream_v2.to_string());
    token_stream.with_metadata(TokenStreamMetadataV1 {
        original_file_path: metadata_v2.original_file_path,
        file_id: metadata_v2.file_id,
    })
}

use cairo_lang_macro::Diagnostic as DiagnosticV2;
use cairo_lang_macro::Severity as SeverityV2;
use cairo_lang_macro_v1::Diagnostic as DiagnosticV1;
use cairo_lang_macro_v1::Severity as SeverityV1;
/// Downcasts new diagnostic struct to the old one
pub fn diagnostic_v2_to_v1(diagnostic_v2: &DiagnosticV2) -> DiagnosticV1 {
    DiagnosticV1 {
        message: diagnostic_v2.message.clone(),
        severity: match diagnostic_v2.severity {
            SeverityV2::Error => SeverityV1::Error,
            SeverityV2::Warning => SeverityV1::Warning,
        },
    }
}
