use cairo_lang_macro_stable::StableExpansionKind;
use std::num::NonZeroU8;

/// Representation of a macro expansion kind.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpansionKind {
    /// `#[proc_macro_name]`
    Attr = 1,
    /// `#[derive(...)]`
    Derive = 2,
    /// `proc_macro_name!(...)`
    Inline = 3,
}

impl ExpansionKind {
    /// Convert to FFI-safe representation.
    /// # Safety
    #[doc(hidden)]
    pub fn into_stable(self) -> StableExpansionKind {
        NonZeroU8::try_from(self as u8).unwrap()
    }

    /// Convert to native Rust representation.
    ///
    /// # Safety
    #[doc(hidden)]
    pub unsafe fn from_stable(kind: &StableExpansionKind) -> Self {
        if *kind == Self::Attr.into_stable() {
            Self::Attr
        } else if *kind == Self::Derive.into_stable() {
            Self::Derive
        } else {
            // Note that it defaults to inline for unknown values.
            Self::Inline
        }
    }
}
