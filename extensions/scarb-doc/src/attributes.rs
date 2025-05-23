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
) -> Vec<String> {
    let mut groups = Vec::new();

    // Retrieve the syntax node associated with the given ID.
    let node = id.stable_location(db).syntax_node(db);

    // Attempt to find the "doc" attribute for this node.
    if let Some(attr) = node.find_attr(db, "doc") {
        // Process the arguments of the "doc" attribute, if any.
        if let OptionArgListParenthesized::ArgListParenthesized(args) = attr.arguments(db) {
            for arg in args.arguments(db).elements(db).iter() {
                if let ArgClause::Unnamed(clause) = arg.arg_clause(db) {
                    let expr = clause.value(db);
                    process_expression(db, expr, &mut groups);
                }
            }
        }
    }

    groups
}

/// Processes an expression, extracting string components into the groups vector.
fn process_expression(db: &ScarbDocDatabase, expr: Expr, groups: &mut Vec<String>) {
    if let Expr::Binary(exp) = expr {
        let expr = exp.rhs(db);
        for child in expr.as_syntax_node().get_children(db) {
            match child.kind(db) {
                SyntaxKind::TokenString => {
                    // Direct string tokens.
                    if let Some(text) = child.text(db) {
                        groups.push(clean_string(&text));
                    }
                }
                SyntaxKind::ExprList => {
                    // Nested list of expressions.
                    for list_item in child.get_children(db) {
                        if matches!(list_item.kind(db), SyntaxKind::TerminalString) {
                            for nested_child in list_item.get_children(db) {
                                if matches!(nested_child.kind(db), SyntaxKind::TokenString) {
                                    if let Some(text) = nested_child.text(db) {
                                        groups.push(clean_string(&text));
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn clean_string(input: &SmolStr) -> String {
    input
        .as_str()
        .replace("\\", "")
        .replace("\"", "")
        .replace("“", "")
        .replace("”", "")
}
