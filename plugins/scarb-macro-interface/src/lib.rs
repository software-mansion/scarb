use std::fmt::Display;

#[doc(hidden)]
pub mod stable_abi;

#[derive(Debug)]
pub enum ProcMacroResult {
    /// Plugin has not taken any action.
    Leave,
    /// Plugin generated [`TokenStream`] replacement.
    Replace(TokenStream),
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
