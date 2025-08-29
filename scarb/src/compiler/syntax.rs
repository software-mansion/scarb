use cairo_lang_syntax::node::SyntaxNode;
use salsa::Database;

pub trait SyntaxNodeExt {
    /// Faster than [`SyntaxNode::tokens`] because we don't travel each leaf, and does not allocate.
    fn for_each_terminal<'db>(&self, db: &'db dyn Database, callback: impl FnMut(&SyntaxNode<'db>))
    where
        Self: 'db;
}

impl<'a> SyntaxNodeExt for SyntaxNode<'a> {
    fn for_each_terminal<'db>(
        &self,
        db: &'db dyn Database,
        mut callback: impl FnMut(&SyntaxNode<'db>),
    ) where
        Self: 'db,
    {
        for_each_terminals_ex(self, db, &mut callback)
    }
}

fn for_each_terminals_ex<'db>(
    node: &SyntaxNode<'db>,
    db: &'db dyn Database,
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
