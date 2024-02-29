use std::fmt::Display;

pub use cairo_lang_macro_attributes::*;

#[doc(hidden)]
pub mod stable_abi;

#[derive(Debug)]
pub enum ProcMacroResult {
    /// Plugin has not taken any action.
    Leave,
    /// Plugin generated [`TokenStream`] replacement.
    Replace {
        token_stream: TokenStream,
        aux_data: Option<AuxData>,
    },
    /// Plugin ordered item removal.
    Remove,
}

#[derive(Debug, Default, Clone)]
pub struct TokenStream(String);

impl TokenStream {
    #[doc(hidden)]
    pub fn new(s: String) -> Self {
        Self(s)
    }
}

impl Display for TokenStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Auxiliary data returned by procedural macro.
#[derive(Debug)]
pub struct AuxData(String);

impl AuxData {
    pub fn new(s: String) -> Self {
        Self(s)
    }

    pub fn try_new<T: serde::Serialize>(value: T) -> Result<Self, serde_json::Error> {
        Ok(Self(serde_json::to_string(&value)?))
    }
}

impl Display for AuxData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
