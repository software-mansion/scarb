use cairo_lang_macro::{TextSpan, Token, TokenStream, TokenStreamMetadata, TokenTree};
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

    pub fn build<'b>(&self) -> TokenStream<'b> {
        let mut token_stream = TokenStream::empty();
        let mut result: Vec<TokenTree<'b>> = Vec::default();
        for node in self.nodes.iter() {
            let leaves = node.tokens(self.db);
            let tokens = leaves
                .map(|node| self.token_from_syntax_node(node.clone()))
                .map(|OwnedToken { content, span }| {
                    // Call to `TokenStream::intern` will copy `content` into an arena allocator
                    // associated with this `TokenStream`.
                    let content = unsafe { token_stream.intern(content.as_str()) };
                    Token { content, span }
                })
                .map(TokenTree::Ident);
            result.extend(tokens);
        }
        token_stream.extend(result);
        match self.metadata.as_ref() {
            Some(metadata) => token_stream.with_metadata(metadata.clone()),
            None => token_stream,
        }
    }

    fn token_from_syntax_node(&self, node: SyntaxNode) -> OwnedToken {
        let span = node.span(self.db).to_str_range();
        OwnedToken {
            content: node.get_text(self.db),
            span: TextSpan {
                start: span.start,
                end: span.end,
            },
        }
    }
}

struct OwnedToken {
    pub content: String,
    pub span: TextSpan,
}
