use std::{ops::Deref, str::FromStr};

use cairo_lang_runner::Arg;
use serde::{de::Visitor, Deserialize};
use serde_json::Value;
use starknet_types_core::felt::Felt as Felt252;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArgsError {
    #[error("failed to parse number: {0}")]
    NumberParseError(#[from] std::num::ParseIntError),
    #[error("failed to parse bigint: {0}")]
    BigIntParseError(#[from] num_bigint::ParseBigIntError),
    #[error("failed to convert slice to array: {0}")]
    ArrayFromSlice(#[from] std::array::TryFromSliceError),
    #[error("number out of range")]
    NumberOutOfRange(#[from] starknet_types_core::felt::FromStrError),
    #[error("failed to parse arguments: {0}")]
    ParseError(#[from] serde_json::Error),
}

/// `Args` is a wrapper around a vector of `Arg`.
///
/// It provides convenience methods for working with a vector of `Arg` and implements
/// `Deref` to allow it to be treated like a vector of `Arg`.
#[derive(Debug)]
pub struct Args(Vec<Arg>);

impl Args {
    /// Creates a new `Args` from a vector of `Arg`.
    ///
    /// # Arguments
    ///
    /// * `args` - A vector of `Arg`.
    ///
    /// # Returns
    ///
    /// * `Args` - A new `Args` instance.
    #[must_use]
    pub fn new(args: Vec<Arg>) -> Self {
        Self(args)
    }
}

impl Clone for Args {
    fn clone(&self) -> Self {
        Self(self.0.iter().map(clone_arg).collect())
    }
}

fn clone_arg(arg: &Arg) -> Arg {
    match arg {
        Arg::Value(value) => Arg::Value(value.to_owned()),
        Arg::Array(args) => Arg::Array(args.iter().map(clone_arg).collect()),
    }
}

impl Deref for Args {
    type Target = Vec<Arg>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Args> for Vec<Arg> {
    fn from(args: Args) -> Self {
        args.0
    }
}

impl From<Vec<Arg>> for Args {
    fn from(args: Vec<Arg>) -> Self {
        Self(args)
    }
}

impl FromStr for Args {
    type Err = ArgsError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let args = serde_json::from_str::<Args>(s)?;
        Ok(args)
    }
}

impl Args {
    fn visit_seq_helper(seq: &[Value]) -> Result<Vec<Arg>, ArgsError> {
        let iterator = seq.iter();
        let mut args = Vec::new();

        for arg in iterator {
            match arg {
                Value::Number(n) => {
                    let n = Felt252::from_str(n.to_string().as_str())?;
                    args.push(Arg::Value(n));
                }
                Value::String(n) => {
                    let n = num_bigint::BigUint::from_str(n)?;
                    let n = n.to_bytes_be();
                    let slice = <&[u8; 32]>::try_from(n.as_slice())?;
                    args.push(Arg::Value(Felt252::from_bytes_be(slice)));
                }
                Value::Array(arr) => {
                    args.push(Arg::Array(Self::visit_seq_helper(arr)?));
                }
                _ => (),
            }
        }

        Ok(args)
    }
}

impl<'de> Visitor<'de> for Args {
    type Value = Args;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a list of arguments")
    }
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut args = Vec::new();
        while let Some(arg) = seq.next_element()? {
            match arg {
                Value::Number(n) => args.push(Value::Number(n)),
                Value::String(n) => args.push(Value::String(n)),
                Value::Array(a) => args.push(Value::Array(a)),
                _ => return Err(serde::de::Error::custom("Invalid type")),
            }
        }

        Self::visit_seq_helper(&args)
            .map(Self::new)
            .map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

impl<'de> Deserialize<'de> for Args {
    fn deserialize<D>(deserializer: D) -> Result<Args, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(Args(Vec::new()))
    }
}
