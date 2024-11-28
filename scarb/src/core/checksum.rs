use std::fmt;
use std::fmt::Write;
use std::io::Read;
use std::str;
use std::str::FromStr;

use anyhow::{bail, ensure, Context, Result};
use data_encoding::{Encoding, HEXLOWER_PERMISSIVE};
use serde::{Deserialize, Serialize};
use sha2::Digest as _;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Checksum([u8; 32]);

impl Checksum {
    const HASH_FUNC_TYPE: &'static str = "sha256";
    const ENCODING: Encoding = HEXLOWER_PERMISSIVE;

    pub fn parse(s: &str) -> Result<Self> {
        fn inner(s: &str) -> Result<Checksum> {
            let Some((hash_func_type, hash)) = s.split_once(':') else {
                bail!("checksum is missing hash function type prefix");
            };

            ensure!(
                hash_func_type == Checksum::HASH_FUNC_TYPE,
                "unsupported hash function type: {hash_func_type}",
            );

            let mut buffer = [0u8; 32];
            let expected_len = buffer.len();

            let decode_len = Checksum::ENCODING.decode_len(hash.len())?;
            ensure!(
                decode_len == expected_len,
                "invalid checksum length {decode_len}, should be {expected_len}"
            );

            let len = Checksum::ENCODING
                .decode_mut(hash.as_bytes(), &mut buffer)
                .map_err(|e| e.error)?;
            ensure!(
                len == expected_len,
                "invalid checksum length {len}, should be {expected_len}"
            );

            Ok(Checksum(buffer))
        }

        inner(s).with_context(|| format!("failed to parse checksum: {s}"))
    }

    /// Create a [`Digest`] instance which will use the same algorithm as this checksum.
    pub fn digest(&self) -> Digest {
        Digest::recommended()
    }
}

impl FromStr for Checksum {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Checksum::parse(s)
    }
}

impl TryFrom<&str> for Checksum {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl TryFrom<String> for Checksum {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<Checksum> for String {
    fn from(c: Checksum) -> Self {
        c.to_string()
    }
}

impl fmt::Display for Checksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(Checksum::HASH_FUNC_TYPE)?;
        f.write_char(':')?;

        let mut buffer = [0u8; 64];
        Checksum::ENCODING.encode_mut(&self.0, &mut buffer);
        // SAFETY: We just generated this hexadecimal string.
        let string = unsafe { str::from_utf8_unchecked(&buffer) };
        f.write_str(string)?;

        Ok(())
    }
}

impl fmt::Debug for Checksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Checksum({self})")
    }
}

pub struct Digest(sha2::Sha256);

impl Digest {
    /// Get recommended digest algorithm.
    pub fn recommended() -> Self {
        Self(sha2::Sha256::new())
    }

    pub fn update(&mut self, bytes: &[u8]) -> &mut Self {
        self.0.update(bytes);
        self
    }

    pub fn update_read(&mut self, mut input: impl Read) -> Result<&mut Self> {
        let mut buf = [0; 64 * 1024];
        loop {
            let n = input.read(&mut buf)?;
            if n == 0 {
                break Ok(self);
            }
            self.update(&buf[..n]);
        }
    }

    pub fn finish(&mut self) -> Checksum {
        Checksum(self.0.finalize_reset().into())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{Checksum, Digest};

    const LOREM: &[u8] =
        b"Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod.";

    fn lorem_checksum() -> Checksum {
        "sha256:b62fc4b9bfbd9310a47d2e595d2c8f468354266be0827aeea9b465d9984908de"
            .parse()
            .unwrap()
    }

    #[test]
    fn checksum_parse_display() {
        let s = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let checksum = Checksum::parse(s).unwrap();
        assert_eq!(
            checksum,
            Checksum([
                0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab,
                0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67,
                0x89, 0xab, 0xcd, 0xef
            ])
        );
        assert_eq!(checksum.to_string(), s);
    }

    #[test]
    fn checksum_serde() {
        let json = r#""sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef""#;
        let checksum = serde_json::from_str::<Checksum>(json).unwrap();
        assert_eq!(
            checksum,
            Checksum([
                0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab,
                0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67,
                0x89, 0xab, 0xcd, 0xef
            ])
        );
        assert_eq!(serde_json::to_string(&checksum).unwrap(), json);
    }

    #[test]
    fn digest() {
        let actual = Digest::recommended().update(LOREM).finish();
        assert_eq!(actual, lorem_checksum());
    }

    #[test]
    fn digest_read() {
        let actual = Digest::recommended()
            .update_read(Cursor::new(LOREM))
            .unwrap()
            .finish();
        assert_eq!(actual, lorem_checksum());
    }
}
