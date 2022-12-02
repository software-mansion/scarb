use std::fmt;

use clap::builder::PossibleValue;
use clap::ValueEnum;
use itertools::Itertools;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum MetadataVersion {
    V1,
}

impl MetadataVersion {
    pub const ALL: [Self; 1] = [Self::V1];

    pub fn numeric(self) -> u8 {
        match self {
            Self::V1 => 1,
        }
    }

    pub fn from_numeric(num: impl Into<u8>) -> Option<Self> {
        match num.into() {
            1u8 => Some(Self::V1),
            _ => None,
        }
    }
}

impl From<MetadataVersion> for u8 {
    fn from(v: MetadataVersion) -> Self {
        v.numeric()
    }
}

impl ValueEnum for MetadataVersion {
    fn value_variants<'a>() -> &'a [Self] {
        &MetadataVersion::ALL
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new(self.numeric().to_string()))
    }
}

impl fmt::Display for MetadataVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.numeric())
    }
}

impl Serialize for MetadataVersion {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(self.numeric())
    }
}

impl<'de> Deserialize<'de> for MetadataVersion {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<MetadataVersion, D::Error> {
        use serde::de::Error;
        let num = u8::deserialize(d)?;
        MetadataVersion::from_numeric(num).ok_or_else(|| {
            let valid_values = MetadataVersion::ALL
                .iter()
                .map(|v| v.numeric().to_string())
                .join(", ");
            Error::custom(format!(
                "unknown metadata version {num}, valid values: {valid_values}"
            ))
        })
    }
}

#[derive(Copy, Clone, Default, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct MetadataVersionPin<const V: u8>;

impl<const V: u8> MetadataVersionPin<V> {
    pub fn numeric(self) -> u8 {
        V
    }

    pub fn from_numeric(num: impl Into<u8>) -> Option<Self> {
        if num.into() == V {
            Some(Self)
        } else {
            None
        }
    }
}

impl<const V: u8> Serialize for MetadataVersionPin<V> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(self.numeric())
    }
}

impl<'de, const V: u8> Deserialize<'de> for MetadataVersionPin<V> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<MetadataVersionPin<V>, D::Error> {
        use serde::de::Error;
        let num = u8::deserialize(d)?;
        MetadataVersionPin::<V>::from_numeric(num)
            .ok_or_else(|| Error::custom("expected metadata version {V}"))
    }
}
