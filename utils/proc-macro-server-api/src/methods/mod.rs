use cairo_lang_macro::TokenStream;
use serde::{Deserialize, Serialize};

pub mod defined_macros;
pub mod expand;

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcMacroResult {
    pub token_stream: TokenStream,
    pub diagnostics: Vec<cairo_lang_macro::Diagnostic>,
}
