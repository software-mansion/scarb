use cairo_lang_syntax::node::SyntaxNode;
use cairo_lang_syntax::node::db::SyntaxGroup;

pub trait SyntaxNodeExt {
    /// Faster than [`SyntaxNode::tokens`] because we don't travel each leaf, and does not allocate.
    fn for_each_terminal<'db>(
        &self,
        db: &'db dyn SyntaxGroup,
        callback: impl FnMut(&SyntaxNode<'db>),
    ) where
        Self: 'db;
}

impl<'a> SyntaxNodeExt for SyntaxNode<'a> {
    fn for_each_terminal<'db>(
        &self,
        db: &'db dyn SyntaxGroup,
        mut callback: impl FnMut(&SyntaxNode<'db>),
    ) where
        Self: 'db,
    {
        for_each_terminals_ex(self, db, &mut callback)
    }
}

fn for_each_terminals_ex<'db>(
    node: &SyntaxNode<'db>,
    db: &'db dyn SyntaxGroup,
    callback: &mut impl FnMut(&SyntaxNode<'db>),
) {
    if node.green_node(db).kind.is_terminal() {
        callback(node);
        return;
    }

    for child in node.get_children(db) {
        for_each_terminals_ex(child, db, callback);
    }
}
