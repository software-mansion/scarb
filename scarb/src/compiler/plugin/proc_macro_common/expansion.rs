use cairo_lang_macro::ExpansionKind as ExpansionKindV1;
use cairo_lang_macro_v2::ExpansionKind as ExpansionKindV2;
use smol_str::SmolStr;

pub const EXEC_ATTR_PREFIX: &str = "__exec_attr_";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpansionKind {
    Attr,
    Derive,
    Inline,
    Executable,
}

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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Expansion {
    pub name: SmolStr,
    pub kind: ExpansionKind,
}

impl Expansion {
    pub fn new(name: impl ToString, kind: ExpansionKind) -> Self {
        Self {
            name: SmolStr::new(name.to_string()),
            kind,
        }
    }
}
