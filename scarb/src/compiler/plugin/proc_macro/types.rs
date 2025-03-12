use cairo_lang_macro::{
    AllocationContext, TextSpan, Token, TokenStream, TokenStreamMetadata, TokenTree,
};
use cairo_lang_syntax::node::{SyntaxNode, db::SyntaxGroup};
use std::ops::Add;

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
                leaves.map(|node| TokenTree::Ident(self.token_from_syntax_node(node.clone(), ctx)))
            })
            .collect();

        match self.metadata.as_ref() {
            Some(metadata) => TokenStream::new(result).with_metadata(metadata.clone()),
            None => TokenStream::new(result),
        }
    }

    pub fn token_from_syntax_node(&self, node: SyntaxNode, ctx: &AllocationContext) -> Token {
        let span = node.span(self.db).to_str_range();
        let text = node.get_text(self.db);
        let span = TextSpan {
            // We skip the whitespace prefix, so that diagnostics start where the actual token contents is.
            start: span.start.add(whitespace_prefix_len(&text)),
            end: span.end,
        };
        Token::new_in(text, span, ctx)
    }
}

fn whitespace_prefix_len(s: &str) -> usize {
    s.chars().take_while(|c| c.is_whitespace()).count()
}

#[cfg(test)]
mod tests {
    use crate::compiler::plugin::proc_macro::TokenStreamBuilder;
    use cairo_lang_macro::{AllocationContext, TextSpan, TokenStream, TokenTree};
    use cairo_lang_parser::utils::SimpleParserDatabase;
    use indoc::indoc;

    #[test]
    fn whitespace_skipped() {
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
                TokenTree::Ident(token) => token,
            }
        };
        let token = token_at(&token_stream, 4);
        assert_eq!(token.content.as_ref(), "{\n");
        assert_eq!(token.span, TextSpan { start: 10, end: 12 });
        let token = token_at(&token_stream, 5);
        assert_eq!(token.content.as_ref(), "    let ");
        // Note we skip 4 whitespaces characters in the span.
        assert_eq!(token.span, TextSpan { start: 16, end: 20 });
        let token = token_at(&token_stream, 6);
        assert_eq!(token.content.as_ref(), "x ");
        assert_eq!(token.span, TextSpan { start: 20, end: 22 });
    }
}
