use std::sync::Arc;

use anyhow::Result;
use scarb_proc_macro_server_types::methods::Method;

use crate::compiler::plugin::collection::WorkspaceProcMacros;

pub mod defined_macros;
pub mod expand_attribute;
pub mod expand_derive;
pub mod expand_inline;

pub trait Handler: Method {
    fn handle(
        proc_macro_host: Arc<WorkspaceProcMacros>,
        params: Self::Params,
    ) -> Result<Self::Response>;
}

use cairo_lang_macro::{
    Diagnostic as DiagnosticV1, Severity as SeverityV1, TokenStream as TokenStreamV1,
    TokenStreamMetadata as TokenStreamMetadataV1,
};
use cairo_lang_macro_v2::{
    Diagnostic as DiagnosticV2, Severity as SeverityV2, TokenStream as TokenStreamV2,
    TokenStreamMetadata as TokenStreamMetadataV2,
};

fn from_v2_metadata(metadata: TokenStreamMetadataV2) -> TokenStreamMetadataV1 {
    TokenStreamMetadataV1 {
        original_file_path: metadata.original_file_path,
        file_id: metadata.file_id,
    }
}

fn from_v2_token_stream(token_stream: TokenStreamV2) -> TokenStreamV1 {
    let metadata = token_stream.metadata.clone();
    TokenStreamV1::new(token_stream.to_string()).with_metadata(from_v2_metadata(metadata))
}

fn from_v1_diagnostic(diagnostic: DiagnosticV1) -> DiagnosticV2 {
    DiagnosticV2 {
        message: diagnostic.message,
        severity: from_v1_severity(diagnostic.severity),
    }
}

fn from_v1_severity(severity: SeverityV1) -> SeverityV2 {
    match severity {
        SeverityV1::Error => SeverityV2::Error,
        SeverityV1::Warning => SeverityV2::Warning,
    }
}
