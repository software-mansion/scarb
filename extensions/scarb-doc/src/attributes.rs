use crate::db::ScarbDocDatabase;
use cairo_lang_defs::ids::TopLevelLanguageElementId;
use cairo_lang_syntax::attribute::structured::{AttributeArgVariant, AttributeStructurize};
use cairo_lang_syntax::node::Terminal;
use cairo_lang_syntax::node::ast::Expr;
use cairo_lang_syntax::node::helpers::QueryAttrs;

/// Extracts string group information from the "doc" attributes of a node.
pub fn find_groups_from_attributes(
    db: &ScarbDocDatabase,
    id: &impl TopLevelLanguageElementId,
) -> Option<String> {
    let node = id.stable_location(db).syntax_node(db);

    if let Some(attr) = node.find_attr(db, "doc") {
        for arg in attr.structurize(db).args {
            if let AttributeArgVariant::Named { value, name } = arg.variant {
                if name.text == "group" {
                    if let Expr::ShortString(value) = value {
                        let text = value.text(db);
                        return Some(text.replace("'", ""));
                    }
                }
            }
        }
    }
    None
}
