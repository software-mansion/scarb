use cairo_lang_macro::TokenStream;
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcMacroResult {
    /// The resultant token stream produced after the macro expansion.
    pub token_stream: TokenStream,
    /// A list of diagnostics produced during the macro execution.
    pub diagnostics: Vec<cairo_lang_macro::Diagnostic>,
}
