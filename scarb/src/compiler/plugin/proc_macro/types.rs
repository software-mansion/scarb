use cairo_lang_macro::{
    AllocationContext, TextSpan, Token, TokenStream, TokenStreamMetadata, TokenTree,
};
use cairo_lang_syntax::node::{db::SyntaxGroup, SyntaxNode};

/// Helps creating TokenStream based on multiple SyntaxNodes,
/// which aren't descendants or ascendants of each other inside the SyntaxTree.
pub struct TokenStreamBuilder<'a> {
    db: &'a dyn SyntaxGroup,
    nodes: Vec<SyntaxNode>,
    metadata: Option<TokenStreamMetadata>,
}

impl<'a, 'b> TokenStreamBuilder<'a> {
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

    pub fn build(&self, ctx: &'b AllocationContext) -> TokenStream<'b> {
        let mut result: Vec<TokenTree<'b>> = Vec::default();
        for node in self.nodes.iter() {
            let leaves = node.tokens(self.db);
            let tokens =
                leaves.map(|node| TokenTree::Ident(self.token_from_syntax_node(node.clone(), ctx)));
            result.extend(tokens);
        }

        match self.metadata.as_ref() {
            Some(metadata) => TokenStream::new(result.clone()).with_metadata(metadata.clone()),
            None => TokenStream::new(result.clone()),
        }
    }

    pub fn token_from_syntax_node(
        &self,
        node: SyntaxNode,
        ctx: &'b AllocationContext,
    ) -> Token<'b> {
        let span = node.span(self.db).to_str_range();
        let content = node.get_text(self.db);
        let content = ctx.intern(content.as_str());
        Token {
            content,
            span: TextSpan {
                start: span.start,
                end: span.end,
            },
        }
    }
}
