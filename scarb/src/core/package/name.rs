use std::borrow::Borrow;
use std::fmt;
use std::str::FromStr;

use anyhow::{Result, bail};
use cairo_lang_filesystem::db::CORELIB_CRATE_NAME;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use crate::internal::restricted_names;

#[cfg(doc)]
use crate::core::Package;

use crate::{
    CAIRO_RUN_PLUGIN_NAME, EXECUTABLE_PLUGIN_NAME, STARKNET_PLUGIN_NAME, TEST_ASSERTS_PLUGIN_NAME,
    TEST_PLUGIN_NAME,
};

/// A [`String`]-like type representing [`Package`] name.
///
/// * Instances of this type are validated upon construction to comply with the
///   [package naming rules](#package-naming-rules).
/// * Values are immutable.
/// * [`Clone`] is `O(1)`.
/// * Short names (which is common for package names) are stack-allocated.
///
/// Package naming rules are described in
/// [Scarb docs](https://docs.swmansion.com/scarb/docs/reference/manifest.html#name).
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(into = "SmolStr", try_from = "SmolStr")]
pub struct PackageName(SmolStr);

impl PackageName {
    pub const CORE: Self = PackageName(SmolStr::new_inline(CORELIB_CRATE_NAME));
    pub const STARKNET: Self = PackageName(SmolStr::new_inline(STARKNET_PLUGIN_NAME));
    pub const EXECUTABLE: Self = PackageName(SmolStr::new_inline(EXECUTABLE_PLUGIN_NAME));
    pub const TEST_PLUGIN: Self = PackageName(SmolStr::new_inline(TEST_PLUGIN_NAME));
    pub const CAIRO_RUN_PLUGIN: Self = PackageName(SmolStr::new_inline(CAIRO_RUN_PLUGIN_NAME));
    pub const TEST_ASSERTS_PLUGIN: Self =
        PackageName(SmolStr::new_inline(TEST_ASSERTS_PLUGIN_NAME));

    /// Constructs and validates new [`PackageName`].
    ///
    /// Panics if name does not conform to package naming rules.
    pub fn new(name: impl AsRef<str>) -> Self {
        Self::try_new(name).unwrap()
    }

    /// Constructs and validates new [`PackageName`].
    pub fn try_new(name: impl AsRef<str>) -> Result<Self> {
        Self::try_new_impl(name.as_ref().into())
    }

    fn try_new_impl(name: SmolStr) -> Result<Self> {
        if name.is_empty() {
            bail!("empty string cannot be used as package name");
        }

        if name == "_" {
            bail!("underscore cannot be used as package name");
        }

        if name != name.to_ascii_lowercase() {
            bail!(
                "invalid package name: `{name}`\n\
                note: usage of ASCII uppercase letters in the package name has been disallowed\n\
                help: change package name to: {}",
                name.to_ascii_lowercase()
            )
        }

        let mut chars = name.chars();

        // Validate first letter.
        if let Some(ch) = chars.next() {
            // A specific error for a potentially common case.
            if ch.is_ascii_digit() {
                bail!(
                    "the name `{name}` cannot be used as a package name, \
                    names cannot start with a digit"
                );
            }

            if !(ch.is_ascii_alphabetic() || ch == '_') {
                bail!(
                    "invalid character `{ch}` in package name: `{name}`, \
                    the first character must be an ASCII lowercase letter or underscore"
                )
            }
        }

        // Validate rest.
        for ch in chars {
            if !(ch.is_ascii_alphanumeric() || ch == '_') {
                bail!(
                    "invalid character `{ch}` in package name: `{name}`, \
                    characters must be ASCII lowercase letters, ASCII numbers or underscore"
                )
            }
        }

        if restricted_names::is_keyword(name.as_str()) {
            bail!(
                "the name `{name}` cannot be used as a package name, \
                names cannot use Cairo keywords see the full list at \
                https://docs.cairo-lang.org/language_constructs/keywords.html"
            )
        }

        Ok(Self(name))
    }

    /// Constructs new [`PackageName`] without validating it to conform to package naming rules.
    ///
    /// # Safety
    /// 1. Does not validate name to conform package naming rules.
    pub unsafe fn new_unchecked(name: impl AsRef<str>) -> Self {
        Self(SmolStr::new(name))
    }

    #[inline(always)]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[inline(always)]
    pub fn to_smol_str(&self) -> SmolStr {
        self.0.clone()
    }
}

impl AsRef<str> for PackageName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<PackageName> for SmolStr {
    fn from(value: PackageName) -> Self {
        value.0
    }
}

impl TryFrom<&str> for PackageName {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        PackageName::try_new(value)
    }
}

impl TryFrom<String> for PackageName {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self> {
        PackageName::try_new(value)
    }
}

impl TryFrom<SmolStr> for PackageName {
    type Error = anyhow::Error;

    fn try_from(value: SmolStr) -> Result<Self> {
        PackageName::try_new(value.as_str())
    }
}

impl FromStr for PackageName {
    type Err = anyhow::Error;

    fn from_str(name: &str) -> Result<Self> {
        PackageName::try_new(name)
    }
}

impl Borrow<str> for PackageName {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<PackageName> for String {
    fn from(value: PackageName) -> Self {
        value.to_string()
    }
}

impl fmt::Display for PackageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for PackageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PackageName({self})")
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use crate::core::PackageName;

    #[test_case("foo")]
    #[test_case("_bar")]
    fn validate_correct_package_name(name: &str) {
        assert!(PackageName::try_new(name).is_ok())
    }

    #[test_case("" => "empty string cannot be used as package name"; "empty string")]
    #[test_case("_" => "underscore cannot be used as package name"; "underscore")]
    #[test_case("1" => "the name `1` cannot be used as a package name, names cannot start with a digit")]
    #[test_case("123" => "the name `123` cannot be used as a package name, names cannot start with a digit")]
    #[test_case("0foo" => "the name `0foo` cannot be used as a package name, names cannot start with a digit")]
    #[test_case("fo-o" => "invalid character `-` in package name: `fo-o`, characters must be ASCII lowercase letters, ASCII numbers or underscore")]
    #[test_case("baR" => "invalid package name: `baR`\nnote: usage of ASCII uppercase letters in the package name has been disallowed\nhelp: change package name to: bar")]
    #[test_case("loop" => "the name `loop` cannot be used as a package name, names cannot use Cairo keywords see the full list at https://docs.cairo-lang.org/language_constructs/keywords.html")]
    #[test_case("as" => "the name `as` cannot be used as a package name, names cannot use Cairo keywords see the full list at https://docs.cairo-lang.org/language_constructs/keywords.html")]
    fn validate_incorrect_package_name(name: &str) -> String {
        PackageName::try_new(name).unwrap_err().to_string()
    }
}
