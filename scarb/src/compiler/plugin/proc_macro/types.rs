use cairo_lang_macro::{Token, TokenStream, TokenStreamMetadata};
use cairo_lang_syntax::node::{db::SyntaxGroup, SyntaxNode};

/// Helps creating TokenStream based on multiple SyntaxNodes,
/// which aren't descendants or ascendants of each other inside the SyntaxTree.
pub struct TokenStreamBuilder<'a> {
    db: &'a dyn SyntaxGroup,
    nodes: Vec<SyntaxNode>,
    metadata: Option<TokenStreamMetadata>,
}

impl<'a> TokenStreamBuilder<'a> {
    pub fn new(db: &'a dyn SyntaxGroup) -> Self {
        Self {
            db,
            nodes: Vec::default(),
            metadata: None,
        }
    }

    pub fn add_node(&mut self, node: SyntaxNode) {
        self.nodes.push(node);
    }

    pub fn with_metadata(&mut self, metadata: TokenStreamMetadata) {
        self.metadata = Some(metadata);
    }

    pub fn build(self) -> TokenStream {
        let mut result: Vec<Token> = Vec::default();
        for node in self.nodes.iter() {
            let leaves = node.tokens(self.db);
            let tokens = leaves
                .iter()
                .map(|node| Token::from_syntax_node(self.db, node.clone()));
            result.extend(tokens);
        }

        match self.metadata {
            Some(metadata) => TokenStream::new(result.clone()).with_metadata(metadata.clone()),
            None => TokenStream::new(result.clone()),
        }
    }
}
