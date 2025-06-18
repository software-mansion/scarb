use cairo_lang_macro::Diagnostic as DiagnosticV2;
use cairo_lang_macro_v1::TokenStream as TokenStreamV1;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub mod defined_macros;
pub mod expand;

pub trait Method {
    const METHOD: &str;

    type Params: Serialize + DeserializeOwned;
    type Response: Serialize + DeserializeOwned;
}

/// Represents the output of a procedural macro execution.
///
/// This struct encapsulates both the resulting token stream from macro expansion
/// and any diagnostic messages (e.g., errors or warnings) that were generated during processing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcMacroResult {
    /// The resultant token stream produced after the macro expansion.
    pub token_stream: TokenStreamV1,
    /// A list of diagnostics produced during the macro execution.
    pub diagnostics: Vec<DiagnosticV2>,
    /// A list of code mappings produced by the macro
    pub code_mappings: Option<Vec<CodeMapping>>,
    /// A list of package ids of macro crates used when resolving this request.
    pub package_ids: Vec<String>,
}

impl Default for ProcMacroResult {
    fn default() -> Self {
        Self {
            token_stream: TokenStreamV1::empty(),
            diagnostics: Vec::new(),
            code_mappings: None,
            package_ids: Default::default(),
        }
    }
}

pub use cairo_lang_macro::{TextOffset, TextSpan};
/// The origin of a code mapping.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodeOrigin {
    /// The origin is a copied node starting at the given offset.
    Start(TextOffset),
    /// The origin was generated from this span, but there's no direct mapping.
    Span(TextSpan),
    /// The origin was generated because of this span, but no code has been copied.
    /// E.g. a macro defined attribute on a function.
    CallSite(TextSpan),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeMapping {
    pub span: TextSpan,
    pub origin: CodeOrigin,
}
