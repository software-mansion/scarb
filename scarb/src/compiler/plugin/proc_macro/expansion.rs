use cairo_lang_macro::ExpansionKind as ExpansionKindV1;
use cairo_lang_macro_v1::ExpansionKind as ExpansionKindV2;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ExpansionKind {
    Attr,
    Derive,
    Inline,
    Executable,
}

// Implement conversions from expansion kind enums exposed by the proc macro implementation api.
// Note that `executable` kind is not represented on the macro side and executable attributes are
// inferred from the attribute expansion by separate logic.
// See `EXEC_ATTR_PREFIX` for implementation details.

impl From<ExpansionKindV1> for ExpansionKind {
    fn from(kind: ExpansionKindV1) -> Self {
        match kind {
            ExpansionKindV1::Attr => Self::Attr,
            ExpansionKindV1::Derive => Self::Derive,
            ExpansionKindV1::Inline => Self::Inline,
        }
    }
}
impl From<ExpansionKindV2> for ExpansionKind {
    fn from(kind: ExpansionKindV2) -> Self {
        match kind {
            ExpansionKindV2::Attr => Self::Attr,
            ExpansionKindV2::Derive => Self::Derive,
            ExpansionKindV2::Inline => Self::Inline,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
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
