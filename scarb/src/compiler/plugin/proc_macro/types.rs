use cairo_lang_macro::{TokenStream, TokenStreamMetadata};
use cairo_lang_syntax::node::SyntaxNode;

pub struct TokenStreamBuilder {
    db: &'a dyn SyntaxGroup,
    nodes: Vec<SyntaxNode>,
    metadata: Option<TokenStreamMetadata>
}

impl TokenStreamBuilder {
    pub fn new(db: &'a dyn SyntaxGroup) -> Self {
        Self {
            db,
            nodes: Vec::default(),
            metadata:: None
        }
    }

    pub fn add_node(mut self, node: SyntaxNode) -> Self {
        self.nodes.push(node);
        self
    }

    pub fn with_metadata(mut self, metadata: TokenStreamMetadata) -> Self {
      self.metadata = Some(metadata);
      self
    }

    pub fn build(self) -> TokenStream {
        let mut tokens: Vec<Token> = Vec::default();
        for node in self.nodes {
            self.get_tree_leaves_as_tokens(tokens, node);
        }
        
        match self.metadata {
          Some(metadata) => TokenStream::new(tokens).with_metadata(metadata),
          None => TokenStream::new(tokens)
        }
    }

    fn get_tree_leaves_as_tokens(self, mut tokens: Vec<Token>, node: SyntaxNode) {
        let children = db.get_children(node);
        if (children.len() == 0) {
            tokens.push(Token::from_syntax_node(db, node))
        } else {
            for child in children {
                traverse_syntax_tree(tokens, db, child);
            }
        }
    }

    fn syntax_node_to_token(self, node: SyntaxNode) -> Token {
        Token::new(
            node.as_syntax_node().get_text(db),
            node.as_syntax_node().span(db),
        )
    }
}

pub trait FromTypedSyntaxNode {
    fn from_typed_syntax_node(db: &dyn SyntaxGroup, node: &impl TypedSyntaxNode) -> Self;
}

pub trait FromSyntaxNode {
    fn from_syntax_node(db: &dyn SyntaxGroup, node: SyntaxNode) -> Self;
}

impl FromTypedSyntaxNode for TokenStream {
    fn from_typed_syntax_node(db: &dyn SyntaxGroup, node: &impl TypedSyntaxNode) -> Self {
        let mut tokens: Vec<Token> = Vec::default();
        let root_node = node.as_syntax_node();

        fn traverse_syntax_tree(mut tokens: Vec<Token>, db: &dyn SyntaxGroup, node: SyntaxNode) {
            let children = db.get_children(node);
            if (children.len() == 0) {
                tokens.push(Token::from_syntax_node(db, node))
            } else {
                for child in children {
                    traverse_syntax_tree(tokens, db, child);
                }
            }
        }
    }
}

impl FromSyntaxNode for Token {
    fn from_syntax_node(db: &dyn SyntaxGroup, node: SyntaxNode) -> Self {
        Token::new(
            node.as_syntax_node().get_text(db),
            node.as_syntax_node().span(db),
        )
    }
}
