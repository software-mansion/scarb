use std::fmt::Display;

use cairo_lang_filesystem::span::TextSpan;
use itertools::Itertools;

#[derive(Debug, Default, Clone)]
pub struct Token {
    span: TextSpan,
    content: String,
}

impl Token {
    pub fn new(content: String, span: TextSpan) -> Self {
        Self { content, span }
    }
}

/// An abstract stream of Cairo tokens.
///
/// This is both input and part of an output of a procedural macro.
#[derive(Debug, Default, Clone)]
pub struct TokenStream {
    tokens: Vec<Token>,
    metadata: TokenStreamMetadata,
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
}

impl TokenStream {
    #[doc(hidden)]
    pub fn new(tokens: Vec<Token>) -> Self {
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
}

impl Display for TokenStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.tokens
                .iter()
                .map(|token| token.content.clone())
                .join("")
        )
    }
}

impl TokenStreamMetadata {
    #[doc(hidden)]
    pub fn new(file_path: impl ToString, file_id: impl ToString) -> Self {
        Self {
            original_file_path: Some(file_path.to_string()),
            file_id: Some(file_id.to_string()),
        }
    }
}
