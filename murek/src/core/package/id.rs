use std::fmt;
use std::ops::Deref;

use anyhow::Result;
use semver::Version;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use smol_str::SmolStr;

use crate::core::source::SourceId;
use crate::internal::static_hash_cache::StaticHashCache;
use crate::internal::to_version::ToVersion;

pub type PackageName = SmolStr;

/// See [`PackageIdInner`] for public fields reference.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PackageId(&'static PackageIdInner);

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub struct PackageIdInner {
    pub name: PackageName,
    pub version: Version,
    pub source_id: SourceId,
}

impl PackageId {
    pub fn new(
        name: impl Into<PackageName>,
        version: impl ToVersion,
        source_id: SourceId,
    ) -> Result<Self> {
        let name = name.into();
        let version = version.to_version()?;
        Ok(Self::pure(name, version, source_id))
    }

    pub fn pure(name: PackageName, version: Version, source_id: SourceId) -> Self {
        static CACHE: StaticHashCache<PackageIdInner> = StaticHashCache::new();
        let inner = PackageIdInner {
            name,
            version,
            source_id,
        };
        Self(CACHE.intern(inner))
    }
}

impl Deref for PackageId {
    type Target = PackageIdInner;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl Serialize for PackageId {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(&format_args!(
            "{} {} ({})",
            self.name,
            self.version,
            self.source_id.to_pretty_url(),
        ))
    }
}

impl<'de> Deserialize<'de> for PackageId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<PackageId, D::Error> {
        use serde::de::Error;

        let string = String::deserialize(d)?;
        let mut s = string.splitn(3, ' ');

        let name = s.next().unwrap().into();

        let Some(version) = s.next() else {
            return Err(Error::custom("invalid serialized PackageId: missing version"));
        };
        let version = version
            .to_version()
            .map_err(|err| Error::custom(format_args!("invalid serialized PackageId: {}", err)))?;

        let Some(url) = s.next() else {
            return Err(Error::custom("invalid serialized PackageId: missing source url"));
        };
        let url = if url.starts_with('(') && url.ends_with(')') {
            &url[1..url.len() - 1]
        } else {
            return Err(Error::custom(
                "invalid serialized PackageId: source url is not wrapped with parentheses",
            ));
        };
        let source_id = SourceId::from_pretty_url(url).map_err(Error::custom)?;

        Ok(PackageId::pure(name, version, source_id))
    }
}

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} v{}", self.name, self.version)?;

        if !self.source_id.is_default_registry() {
            write!(f, " ({})", self.source_id)?;
        }

        Ok(())
    }
}

impl fmt::Debug for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PackageId({} {} {})",
            self.name, self.version, self.source_id
        )
    }
}

#[cfg(test)]
mod tests {
    use serde_test::{assert_de_tokens_error, assert_tokens, Token};
    use test_case::test_case;

    use crate::core::package::PackageId;
    use crate::core::source::SourceId;

    fn leak_string(string: String) -> &'static str {
        Box::leak(string.into_boxed_str())
    }

    #[test_case(SourceId::mock_git())]
    #[test_case(SourceId::mock_path())]
    fn serialization(source_id: SourceId) {
        let pkg_id = PackageId::new("foo", "1.0.0", source_id).unwrap();
        let expected = format!("foo 1.0.0 ({})", source_id.to_pretty_url());
        assert_tokens(&pkg_id, &[Token::Str(leak_string(expected))]);
    }

    #[test_case("foo", "invalid serialized PackageId: missing version")]
    #[test_case("foo ", "invalid serialized PackageId: cannot parse '' as a semver")]
    #[test_case(
        "foo 1.0",
        "invalid serialized PackageId: cannot parse '1.0' as a semver"
    )]
    #[test_case(
        "foo v1.0.0",
        "invalid serialized PackageId: cannot parse 'v1.0.0' as a semver"
    )]
    fn deserialize_errors(ser: &'static str, err: &'static str) {
        assert_de_tokens_error::<PackageId>(&[Token::Str(ser)], err);
    }

    #[test]
    fn display() {
        let source_id = SourceId::mock_path();
        let pkg_id = PackageId::new("foo", "1.0.0", source_id).unwrap();
        assert_eq!(format!("foo v1.0.0 ({source_id})"), pkg_id.to_string());
    }
}
