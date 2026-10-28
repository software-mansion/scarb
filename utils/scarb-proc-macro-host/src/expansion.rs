use cairo_lang_macro::ExpansionKind as MacroExpansionKind;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum ExpansionKind {
    Attr,
    Derive,
    Inline,
    Executable,
}

// Implement conversion from the expansion kind enum exposed by the procedural macro
// implementation api.
// Note that `executable` kind is not represented on the macro side and executable attributes are
// inferred from the attribute expansion by separate logic.
// See `EXEC_ATTR_PREFIX` for implementation details.

impl From<MacroExpansionKind> for ExpansionKind {
    fn from(kind: MacroExpansionKind) -> Self {
        match kind {
            MacroExpansionKind::Attr => Self::Attr,
            MacroExpansionKind::Derive => Self::Derive,
            MacroExpansionKind::Inline => Self::Inline,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Expansion {
    /// Name of the expansion function as defined in the macro source code.
    pub expansion_name: SmolStr,
    /// Name of the macro as available to the user through Cairo code.
    /// This is equivalent to `expansion_name` with potentially changed casing.
    pub cairo_name: SmolStr,
    pub kind: ExpansionKind,
}

impl Expansion {
    pub fn matches_query(&self, query: &ExpansionQuery) -> bool {
        match query {
            ExpansionQuery::WithCairoName { cairo_name, kind } => {
                *cairo_name == self.cairo_name && self.kind == *kind
            }
            ExpansionQuery::WithExpansionName {
                expansion_name,
                kind,
            } => *expansion_name == self.expansion_name && self.kind == *kind,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExpansionQuery {
    WithCairoName {
        cairo_name: SmolStr,
        kind: ExpansionKind,
    },
    WithExpansionName {
        expansion_name: SmolStr,
        kind: ExpansionKind,
    },
}

impl ExpansionQuery {
    pub fn with_cairo_name(name: impl ToString, kind: ExpansionKind) -> Self {
        Self::WithCairoName {
            cairo_name: SmolStr::new(name.to_string()),
            kind,
        }
    }

    pub fn with_expansion_name(name: impl ToString, kind: ExpansionKind) -> Self {
        Self::WithExpansionName {
            expansion_name: SmolStr::new(name.to_string()),
            kind,
        }
    }
}
