use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[cfg(doc)]
use super::Metadata;

const METADATA_VERSION: u64 = {
    // `FromStr` is not `const`, so we do poor man's parsing ourselves here.
    let mut bytes = env!("CARGO_PKG_VERSION_MAJOR").as_bytes();
    let mut num = 0u64;
    while let [ch @ b'0'..=b'9', rem @ ..] = bytes {
        bytes = rem;
        num *= 10;
        num += (*ch - b'0') as u64;
    }
    num
};

/// A zero-sized type enforcing [`serde`] to serialize/deserialize it to a constant number
/// representing version of [`Metadata`] schema.
///
/// The version number corresponds to the major version of this crate.
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct VersionPin;

impl VersionPin {
    /// Get version as a number.
    pub const fn numeric(self) -> u64 {
        METADATA_VERSION
    }

    /// Construct this pin if `num` equals to the pinned version.
    pub const fn from_numeric(num: u64) -> Option<Self> {
        if num == Self.numeric() {
            Some(Self)
        } else {
            None
        }
    }
}

impl Serialize for VersionPin {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(self.numeric())
    }
}

impl<'de> Deserialize<'de> for VersionPin {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<VersionPin, D::Error> {
        use serde::de::Error;
        let num = u64::deserialize(d)?;
        VersionPin::from_numeric(num)
            .ok_or_else(|| Error::custom(format!("expected metadata version {}", Self.numeric())))
    }
}

impl From<VersionPin> for u64 {
    fn from(pin: VersionPin) -> Self {
        pin.numeric()
    }
}

impl fmt::Display for VersionPin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Self.numeric())
    }
}
