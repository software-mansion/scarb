use cairo_lang_macro::TokenStream;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub mod defined_macros;
pub mod expand;

pub trait Method {
    const METHOD: &str;

    type Params: Serialize + DeserializeOwned;
    type Response: Serialize + DeserializeOwned;
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcMacroResult {
    pub token_stream: TokenStream,
    pub diagnostics: Vec<cairo_lang_macro::Diagnostic>,
}
