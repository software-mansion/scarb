use std::{fmt::Display, iter::once};

use cairo_lang_stable_token::{StableSpan, StableToken, ToStableTokenStream};

/// An abstract stream of Cairo tokens.
///
/// This is both input and part of an output of a procedural macro.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct TokenStream {
    pub tokens: Vec<TokenTree>,
    pub metadata: TokenStreamMetadata,
}

/// A single token or a delimited sequence of token trees.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenTree {
    Ident(Token),
}

impl Default for TokenTree {
    fn default() -> Self {
        Self::Ident(Default::default())
    }
}

/// A range of text offsets that form a span (like text selection).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
}

/// A single Cairo token.
///
/// The most atomic item of Cairo code representation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Token {
    pub content: String,
    pub span: Option<TextSpan>,
}

/// Metadata of [`TokenStream`].
///
/// This struct describes the origin of the [`TokenStream`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct TokenStreamMetadata {
    /// The path to the file from which the [`TokenStream`] has been created.
    pub original_file_path: Option<String>,
    /// ID of the file from which the [`TokenStream`] has been created.
    ///
    /// It is guaranteed, that the `file_id` will be unique for each file.
    pub file_id: Option<String>,
    /// Cairo edition defined for the token stream.
    pub edition: Option<String>,
}

impl TokenStream {
    #[doc(hidden)]
    pub fn new(tokens: Vec<TokenTree>) -> Self {
        Self {
            tokens,
            metadata: TokenStreamMetadata::default(),
        }
    }

    #[doc(hidden)]
    pub fn empty() -> Self {
        Self::new(Vec::default())
    }

    #[doc(hidden)]
    pub fn with_metadata(mut self, metadata: TokenStreamMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Get `[TokenStreamMetadata`] associated with this [`TokenStream`].
    ///
    /// The metadata struct can be used to describe the [`TokenStream`] origin.
    pub fn metadata(&self) -> &TokenStreamMetadata {
        &self.metadata
    }

    pub fn is_empty(&self) -> bool {
        self.to_string().is_empty()
    }

    pub fn from_stable_token_stream(
        stable_token_stream: impl Iterator<Item = StableToken>,
    ) -> Self {
        Self::new(
            stable_token_stream
                .map(|stable_token| {
                    TokenTree::Ident(Token::new(
                        stable_token.0,
                        stable_token.1.map(|stable_span| TextSpan {
                            start: stable_span.start,
                            end: stable_span.end,
                        }),
                    ))
                })
                .collect(),
        )
    }

    pub fn extend(&mut self, token_stream: Self) {
        self.tokens.extend(token_stream.tokens);
    }

    pub fn push_token(&mut self, token_tree: TokenTree) {
        self.tokens.push(token_tree);
    }
}

impl Display for TokenStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for token in &self.tokens {
            match token {
                TokenTree::Ident(token) => {
                    write!(f, "{}", token.content.clone())?;
                }
            }
        }
        Ok(())
    }
}

impl TokenStreamMetadata {
    #[doc(hidden)]
    pub fn new(file_path: impl ToString, file_id: impl ToString, edition: impl ToString) -> Self {
        Self {
            original_file_path: Some(file_path.to_string()),
            file_id: Some(file_id.to_string()),
            edition: Some(edition.to_string()),
        }
    }
}

impl TokenTree {
    pub fn from_ident(token: Token) -> Self {
        Self::Ident(token)
    }
}

impl TextSpan {
    pub fn new(start: usize, end: usize) -> TextSpan {
        TextSpan { start, end }
    }
}

impl Token {
    pub fn new(content: String, span: Option<TextSpan>) -> Self {
        Self { content, span }
    }
}

impl ToStableTokenStream for TokenStream {
    fn to_stable_token_stream(&self) -> impl Iterator<Item = StableToken> {
        self.tokens
            .clone()
            .into_iter()
            .map(|token_tree| match token_tree {
                TokenTree::Ident(token) => StableToken::new(
                    token.content,
                    token.span.map(|span| StableSpan {
                        start: span.start,
                        end: span.end,
                    }),
                ),
            })
    }
}

impl ToStableTokenStream for TokenTree {
    fn to_stable_token_stream(&self) -> impl Iterator<Item = StableToken> {
        once(match self {
            TokenTree::Ident(token) => StableToken::new(
                token.content.clone(),
                token.span.clone().map(|span| StableSpan {
                    start: span.start,
                    end: span.end,
                }),
            ),
        })
    }
}
