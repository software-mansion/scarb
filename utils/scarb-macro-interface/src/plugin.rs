use std::fmt::Display;

#[derive(Debug)]
#[allow(dead_code)]
pub enum ProcMacroResult {
    /// Plugin has not taken any action.
    Leave,
    /// Plugin generated TokenStream replacement.
    Replace(TokenStream),
    /// Plugin ordered item removal.
    Remove,
}

#[derive(Debug, Default, Clone)]
pub struct TokenStream(String);

impl From<String> for TokenStream {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl Display for TokenStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
