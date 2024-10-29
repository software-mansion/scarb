use bumpalo::Bump;
use std::fmt::Display;

/// An abstract stream of Cairo tokens.
///
/// This is both input and part of an output of a procedural macro.
#[derive(Debug)]
pub struct TokenStream<'a> {
    pub tokens: Vec<TokenTree<'a>>,
    pub metadata: TokenStreamMetadata,
}

/// A single token or a delimited sequence of token trees.
#[derive(Debug, Clone)]
pub enum TokenTree<'a> {
    Ident(Token<'a>),
}

/// A range of text offsets that form a span (like text selection).
#[derive(Debug, Default, Clone)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
}

/// A single Cairo token.
///
/// The most atomic item, of Cairo code representation, when passed between macro and host.
#[derive(Debug, Default, Clone)]
pub struct Token<'a> {
    pub content: &'a str,
    pub span: TextSpan,
}

pub struct AllocationContext {
    bump: Bump,
}

impl AllocationContext {
    pub fn intern(&self, value: &str) -> &str {
        self.bump.alloc_str(value)
    }
}

impl Default for AllocationContext {
    fn default() -> Self {
        Self { bump: Bump::new() }
    }
}

impl Drop for AllocationContext {
    fn drop(&mut self) {
        self.bump.reset();
    }
}

/// Metadata of [`TokenStream`].
///
/// This struct can be used to describe the origin of the [`TokenStream`].
#[derive(Debug, Default, Clone)]
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

impl<'a> TokenStream<'a> {
    #[doc(hidden)]
    pub fn new(tokens: Vec<TokenTree<'a>>) -> Self {
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
        self.tokens.is_empty()
    }
}

impl Display for TokenStream<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for token in &self.tokens {
            match token {
                TokenTree::Ident(token) => {
                    write!(f, "{}", token.content)?;
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

impl<'a> TokenTree<'a> {
    pub fn from_ident(token: Token<'a>) -> Self {
        Self::Ident(token)
    }
}

impl TextSpan {
    pub fn new(start: usize, end: usize) -> TextSpan {
        TextSpan { start, end }
    }
}
