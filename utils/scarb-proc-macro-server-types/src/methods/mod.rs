use std::fmt::Display;

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

/// A single token of an expansion result, together with the span of the code it came from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpannedToken {
    pub content: String,
    pub span: TextSpan,
}

/// The code produced by a macro expansion.
///
/// This mirrors [`cairo_lang_macro::TokenStream`], but as plain owned data. The token stream of
/// the procedural macro api interns its strings in a thread-local arena, which makes it unsuitable
/// both for crossing a process boundary and for being stored by the caller.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpannedTokenStream(pub Vec<SpannedToken>);

impl SpannedTokenStream {
    /// Represents code that carries no span information of its own.
    ///
    /// The whole content is reported as a single token attributed to `origin`, so that the caller
    /// maps all of it back onto that one piece of the original source. Empty content yields an
    /// empty stream, which asks the caller to remove the expanded item.
    pub fn unspanned(content: impl Into<String>, origin: TextSpan) -> Self {
        let content = content.into();
        if content.is_empty() {
            return Self::default();
        }
        Self(vec![SpannedToken {
            content,
            span: origin,
        }])
    }

    /// An empty stream, which asks the caller to remove the expanded item.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
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

    /// Converts into a token stream the procedural macro api understands, allocating the token
    /// contents in `ctx`.
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

impl Display for SpannedTokenStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for token in &self.0 {
            f.write_str(&token.content)?;
        }
        Ok(())
    }
}

/// Represents the output of a procedural macro execution.
///
/// This struct encapsulates both the resulting token stream from macro expansion
/// and any diagnostic messages (e.g., errors or warnings) that were generated during processing.
///
/// Each token of the result carries the span of the code it originates from, which is what the
/// caller uses to map expanded code back onto the original source. Expansions performed through
/// the v1 procedural macro api, which has no notion of spans, are reported as a single token
/// covering the whole expansion origin.
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
    fn unspanned_content_is_attributed_to_its_origin() {
        let stream = SpannedTokenStream::unspanned("fn foo() {}", span(3, 14));
        assert_eq!(
            stream.0,
            vec![SpannedToken {
                content: "fn foo() {}".to_string(),
                span: span(3, 14)
            }]
        );
        assert!(!stream.is_empty());
        assert_eq!(stream.to_string(), "fn foo() {}");
    }

    #[test]
    fn empty_content_produces_an_empty_stream() {
        // An expansion that returns nothing asks the caller to remove the original item, so the
        // emptiness has to survive being wrapped into a single token.
        let stream = SpannedTokenStream::unspanned("", span(0, 10));
        assert!(stream.is_empty());
        assert_eq!(stream.to_string(), "");
    }

    #[test]
    fn default_stream_is_empty() {
        assert!(SpannedTokenStream::default().is_empty());
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

    #[test]
    fn round_trips_through_serde() {
        let original = SpannedTokenStream::unspanned("impl A {}", span(2, 20));
        let json = serde_json::to_string(&original).unwrap();
        let back: SpannedTokenStream = serde_json::from_str(&json).unwrap();
        assert_eq!(back, original);
    }

    #[test]
    fn display_concatenates_token_contents() {
        let stream = SpannedTokenStream(vec![
            SpannedToken {
                content: "a".to_string(),
                span: span(0, 1),
            },
            SpannedToken {
                content: "b".to_string(),
                span: span(5, 6),
            },
        ]);
        // Tokens need not be contiguous; the rendered code is just their contents in order.
        assert_eq!(stream.to_string(), "ab");
    }
}
