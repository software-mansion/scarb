use std::fmt::Display;

use cairo_lang_filesystem::span::{TextOffset, TextSpan, TextWidth};
use cairo_lang_parser::utils::SimpleParserDatabase;
use cairo_lang_syntax::node::{db::SyntaxGroup, SyntaxNode};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

/// Representation of most atomic struct that is passed within host<->macro communication.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Token {
    span: TextSpan,
    content: String,
}

impl Token {
    pub fn new(content: String, span: TextSpan) -> Self {
        Self { content, span }
    }

    pub fn from_syntax_node(db: &dyn SyntaxGroup, node: SyntaxNode) -> Self {
        Token::new(node.get_text(db), node.span(db))
    }
}

/// An abstract stream of Cairo tokens.
///
/// This is both input and part of an output of a procedural macro.
#[derive(Debug, Default, Clone)]
pub struct TokenStream {
    pub tokens: Vec<Token>,
    pub metadata: TokenStreamMetadata,
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

    pub fn from_string(str: String) -> Self {
        let db = SimpleParserDatabase::default();
        // Sometimes this will return an error, when the user pass the simple literal value like "34".
        // In this case, we create the Token manually with its span.
        let node = db.parse_virtual(str.clone());

        let tokens = match node {
            Ok(node) => {
                let nodes = node.tokens(&db);
                nodes
                    .iter()
                    .map(|node| Token::from_syntax_node(&db, node.clone()))
                    .collect()
            }
            Err(_) => vec![Token::new(
                str.clone(),
                TextSpan {
                    start: TextOffset::default(),
                    end: TextOffset::default().add_width(TextWidth::from_str(&str)),
                },
            )],
        };

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
