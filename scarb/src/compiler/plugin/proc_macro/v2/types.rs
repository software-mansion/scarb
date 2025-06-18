use cairo_lang_filesystem::span::TextWidth;
use cairo_lang_macro::{
    AllocationContext, TextSpan, Token, TokenStream, TokenStreamMetadata, TokenTree,
};
use cairo_lang_syntax::node::{SyntaxNode, db::SyntaxGroup};

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

    pub fn build(&self, ctx: &AllocationContext) -> TokenStream {
        let result: Vec<TokenTree> = self
            .nodes
            .iter()
            .flat_map(|node| {
                let leaves = node.tokens(self.db);
                leaves
                    .flat_map(|node| self.token_from_syntax_node(node, ctx))
                    .map(TokenTree::Ident)
            })
            .collect();

        match self.metadata.as_ref() {
            Some(metadata) => TokenStream::new(result).with_metadata(metadata.clone()),
            None => TokenStream::new(result),
        }
    }

    pub fn token_from_syntax_node(&self, node: SyntaxNode, ctx: &AllocationContext) -> Vec<Token> {
        let span_without_trivia = node.span_without_trivia(self.db);
        let span_with_trivia = node.span(self.db);
        let text = node.get_text(self.db);
        let mut result = Vec::new();
        let prefix_len = span_without_trivia.start - span_with_trivia.start;
        let (prefix, rest) = text.split_at(prefix_len.as_u32() as usize);
        if prefix_len > TextWidth::ZERO {
            result.push(Token::new_in(
                prefix,
                TextSpan {
                    start: span_with_trivia.start.as_u32(),
                    end: span_without_trivia.start.as_u32(),
                },
                ctx,
            ))
        }
        let suffix_len = span_with_trivia.end - span_without_trivia.end;
        let (content, suffix) = rest.split_at(rest.len() - suffix_len.as_u32() as usize);
        if !content.is_empty() {
            result.push(Token::new_in(
                content,
                TextSpan {
                    start: span_without_trivia.start.as_u32(),
                    end: span_without_trivia.end.as_u32(),
                },
                ctx,
            ));
        }
        if suffix_len > TextWidth::ZERO {
            result.push(Token::new_in(
                suffix,
                TextSpan {
                    start: span_without_trivia.end.as_u32(),
                    end: span_with_trivia.end.as_u32(),
                },
                ctx,
            ));
        }
        result
    }
}

impl Extend<SyntaxNode> for TokenStreamBuilder<'_> {
    fn extend<T: IntoIterator<Item = SyntaxNode>>(&mut self, iter: T) {
        for node in iter {
            self.add_node(node);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::compiler::plugin::proc_macro::v2::TokenStreamBuilder;
    use cairo_lang_macro::{AllocationContext, TextSpan, TokenStream, TokenTree};
    use cairo_lang_parser::utils::SimpleParserDatabase;
    use indoc::indoc;

    #[test]
    fn tokens_built_correctly() {
        let db = SimpleParserDatabase::default();
        let mut builder = TokenStreamBuilder::new(&db);
        let content = indoc! {r#"
            fn main() {
                let x = 42;
            }
        "#};
        let parsed = db.parse_virtual(content).unwrap();
        builder.add_node(parsed);
        let ctx = AllocationContext::default();
        let token_stream = builder.build(&ctx);
        let token_at = |token_stream: &TokenStream, idx: usize| {
            let token: TokenTree = token_stream.tokens[idx].clone();
            match token {
                TokenTree::Ident(token) => (token.content.as_ref().to_string(), token.span.clone()),
            }
        };
        assert_eq!(token_stream.tokens.len(), 20);
        assert_eq!(
            token_at(&token_stream, 0),
            ("fn".to_string(), TextSpan { start: 0, end: 2 }),
        );
        assert_eq!(
            token_at(&token_stream, 1),
            (" ".to_string(), TextSpan { start: 2, end: 3 }),
        );
        assert_eq!(
            token_at(&token_stream, 2),
            ("main".to_string(), TextSpan { start: 3, end: 7 }),
        );
        assert_eq!(
            token_at(&token_stream, 3),
            ("(".to_string(), TextSpan { start: 7, end: 8 }),
        );
        assert_eq!(
            token_at(&token_stream, 4),
            (")".to_string(), TextSpan { start: 8, end: 9 }),
        );
        assert_eq!(
            token_at(&token_stream, 5),
            (" ".to_string(), TextSpan { start: 9, end: 10 }),
        );
        assert_eq!(
            token_at(&token_stream, 6),
            ("{".to_string(), TextSpan { start: 10, end: 11 }),
        );
        assert_eq!(
            token_at(&token_stream, 7),
            ("\n".to_string(), TextSpan { start: 11, end: 12 }),
        );
        assert_eq!(
            token_at(&token_stream, 8),
            ("    ".to_string(), TextSpan { start: 12, end: 16 }),
        );
        assert_eq!(
            token_at(&token_stream, 9),
            ("let".to_string(), TextSpan { start: 16, end: 19 }),
        );
        assert_eq!(
            token_at(&token_stream, 10),
            (" ".to_string(), TextSpan { start: 19, end: 20 }),
        );
        assert_eq!(
            token_at(&token_stream, 11),
            ("x".to_string(), TextSpan { start: 20, end: 21 }),
        );
        assert_eq!(
            token_at(&token_stream, 12),
            (" ".to_string(), TextSpan { start: 21, end: 22 }),
        );
        assert_eq!(
            token_at(&token_stream, 13),
            ("=".to_string(), TextSpan { start: 22, end: 23 }),
        );
        assert_eq!(
            token_at(&token_stream, 14),
            (" ".to_string(), TextSpan { start: 23, end: 24 }),
        );
        assert_eq!(
            token_at(&token_stream, 15),
            ("42".to_string(), TextSpan { start: 24, end: 26 }),
        );
        assert_eq!(
            token_at(&token_stream, 16),
            (";".to_string(), TextSpan { start: 26, end: 27 }),
        );
        assert_eq!(
            token_at(&token_stream, 17),
            ("\n".to_string(), TextSpan { start: 27, end: 28 }),
        );
        assert_eq!(
            token_at(&token_stream, 18),
            ("}".to_string(), TextSpan { start: 28, end: 29 }),
        );
        assert_eq!(
            token_at(&token_stream, 19),
            ("\n".to_string(), TextSpan { start: 29, end: 30 }),
        );
    }
}
