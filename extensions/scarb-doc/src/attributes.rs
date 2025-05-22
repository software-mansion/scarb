use crate::db::ScarbDocDatabase;
use cairo_lang_defs::ids::TopLevelLanguageElementId;
use cairo_lang_syntax::node::TypedSyntaxNode;
use cairo_lang_syntax::node::ast::{ArgClause, Expr, OptionArgListParenthesized};
use cairo_lang_syntax::node::helpers::QueryAttrs;
use cairo_lang_syntax::node::kind::SyntaxKind;
use smol_str::SmolStr;

/// Extracts string group information from the "doc" attributes of a node.
pub fn find_groups_from_attributes(
    db: &ScarbDocDatabase,
    id: &impl TopLevelLanguageElementId,
) -> Option<String> {
    let node = id.stable_location(db).syntax_node(db);

    if let Some(attr) = node.find_attr(db, "doc") {
        // Process the arguments of the "doc" attribute, if any.
        if let OptionArgListParenthesized::ArgListParenthesized(args) = attr.arguments(db) {
            for arg in args.arguments(db).elements(db).iter() {
                if let ArgClause::Unnamed(clause) = arg.arg_clause(db) {
                    let expr = clause.value(db);
                    return process_expression(db, expr);
                }
            }
        }
    }
    None
}

/// Processes an expression, extracting string components into the groups vector.
fn process_expression(db: &ScarbDocDatabase, expr: Expr) -> Option<String> {
    if let Expr::Binary(exp) = expr {
        let expr = exp.rhs(db);
        for child in expr.as_syntax_node().get_children(db).iter() {
            if child.kind(db) == SyntaxKind::TokenString {
                if let Some(text) = child.text(db) {
                    return Some(clean_string(&text));
                }
            }
        }
    }
    None
}

fn clean_string(input: &SmolStr) -> String {
    input
        .as_str()
        .replace("\\", "")
        .replace("\"", "")
        .replace("“", "")
        .replace("”", "")
}
