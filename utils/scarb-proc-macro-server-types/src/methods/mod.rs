use cairo_lang_macro::{AllocationContext, Diagnostic, Token, TokenStream, TokenTree};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub mod defined_macros;
pub mod expand;

pub use cairo_lang_macro::{TextOffset, TextSpan};

pub trait Method {
    const METHOD: &str;

    type Params: Serialize + DeserializeOwned;
    type Response: Serialize + DeserializeOwned;
}

/// A piece of an expansion result, together with the span of the code it came from.
///
/// For v2 macros it is a single token; a v1 expansion is one piece spanning its origin.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpannedToken {
    pub content: String,
    pub span: TextSpan,
}

/// The code produced by a macro expansion, as plain owned data.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpannedTokenStream(pub Vec<SpannedToken>);

impl SpannedTokenStream {
    /// Wraps code without span information into a single token spanning `origin`.
    pub fn from_unspanned(content: impl Into<String>, origin: TextSpan) -> Self {
        let content = content.into();
        if content.is_empty() {
            return Self::default();
        }
        Self(vec![SpannedToken {
            content,
            span: origin,
        }])
    }

    /// Converts a token stream produced by the procedural macro api.
    pub fn from_token_stream(token_stream: &TokenStream) -> Self {
        Self(
            token_stream
                .tokens
                .iter()
                .map(|token| {
                    let TokenTree::Ident(token) = token;
                    SpannedToken {
                        content: token.content.as_ref().to_string(),
                        span: token.span.clone(),
                    }
                })
                .collect(),
        )
    }

    /// Converts into a token stream of the procedural macro api.
    pub fn to_token_stream(&self, ctx: &AllocationContext) -> TokenStream {
        TokenStream::new(
            self.0
                .iter()
                .map(|token| {
                    TokenTree::Ident(Token::new_in(&token.content, token.span.clone(), ctx))
                })
                .collect(),
        )
    }
}

/// Represents the output of a procedural macro execution.
///
/// This struct encapsulates both the resulting token stream from macro expansion
/// and any diagnostic messages (e.g., errors or warnings) that were generated during processing.
///
/// Expansions of v1 macros are reported as a single token covering the whole expansion origin.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcMacroResult {
    /// The resultant token stream produced after the macro expansion.
    pub token_stream: SpannedTokenStream,
    /// A list of diagnostics produced during the macro execution.
    pub diagnostics: Vec<Diagnostic>,
    /// A proc macro fingerprint
    pub fingerprint: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(start: u32, end: u32) -> TextSpan {
        TextSpan::new(start, end)
    }

    #[test]
    fn round_trips_through_the_macro_api_token_stream() {
        let original = SpannedTokenStream(vec![
            SpannedToken {
                content: "fn ".to_string(),
                span: span(0, 3),
            },
            SpannedToken {
                content: "foo".to_string(),
                span: span(3, 6),
            },
            SpannedToken {
                content: "() {}".to_string(),
                span: span(6, 11),
            },
        ]);

        let ctx = AllocationContext::default();
        let token_stream = original.to_token_stream(&ctx);
        assert_eq!(token_stream.to_string(), "fn foo() {}");

        let back = SpannedTokenStream::from_token_stream(&token_stream);
        assert_eq!(back, original);
    }
}
