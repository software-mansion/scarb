use crate::db::ScarbDocDatabase;
use crate::diagnostics::add_diagnostic_message;
use cairo_lang_defs::ids::TopLevelLanguageElementId;
use cairo_lang_syntax::attribute::structured::{AttributeArgVariant, AttributeStructurize};
use cairo_lang_syntax::node::Terminal;
use cairo_lang_syntax::node::ast::Expr;
use cairo_lang_syntax::node::helpers::QueryAttrs;

/// Extracts string group information from the "doc" attributes of a node.
pub fn find_groups_from_attributes<'db>(
    db: &'db ScarbDocDatabase,
    id: &impl TopLevelLanguageElementId<'db>,
) -> Option<String> {
    let node = id.stable_location(db).syntax_node(db);

    if let Some(attr) = node.find_attr(db, "doc") {
        for arg in attr.structurize(db).args {
            let text = arg.text(db);
            if let AttributeArgVariant::Named { value, name } = arg.variant {
                if name.text.to_string(db) == "group" {
                    if let Expr::String(value) = value {
                        let text = value.text(db);
                        return Some(text.to_string(db).replace("\"", ""));
                    } else {
                        let diagnostic_message = format!(
                            "Invalid attribute `{}` in {}.\nUse `group: \"group name\"` instead.",
                            text,
                            id.full_path(db),
                        );
                        add_diagnostic_message(diagnostic_message);
                    }
                } else {
                    let diagnostic_message = format!(
                        "Invalid attribute `{}` in {}.\nUse `group: \"group name\"` instead.",
                        text,
                        id.full_path(db),
                    );
                    add_diagnostic_message(diagnostic_message);
                }
            } else {
                let diagnostic_message = format!(
                    "Invalid attribute `#doc({})]` in {}.\nUse `#[doc(group: \"group name\")]'` or `#[doc(hidden)]`, instead",
                    text,
                    id.full_path(db)
                );
                add_diagnostic_message(diagnostic_message);
            }
        }
    }
    None
}
