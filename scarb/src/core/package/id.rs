use std::fmt;
use std::ops::Deref;

use anyhow::Result;
use semver::Version;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::core::source::SourceId;
use crate::core::PackageName;
use crate::internal::static_hash_cache::StaticHashCache;
use crate::internal::to_version::ToVersion;

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
    pub fn new(name: PackageName, version: Version, source_id: SourceId) -> Self {
        static CACHE: StaticHashCache<PackageIdInner> = StaticHashCache::new();
        let inner = PackageIdInner {
            name,
            version,
            source_id,
        };
        Self(CACHE.intern(inner))
    }

    pub fn is_core(&self) -> bool {
        self.name == PackageName::CORE && self.source_id == SourceId::for_std()
    }

    #[cfg(test)]
    pub(crate) fn from_display_str(string: &str) -> Result<Self> {
        use anyhow::{anyhow, bail, Context};

        let mut s = string.splitn(3, ' ');

        let name =
            PackageName::try_new(s.next().unwrap()).context("invalid displayed PackageId")?;

        let Some(version) = s.next() else {
            bail!("invalid displayed PackageId: missing version");
        };
        let Some(version) = version.strip_prefix('v') else {
            bail!("invalid displayed PackageId: version does not start with letter `v`");
        };
        let version = version
            .to_version()
            .map_err(|err| anyhow!("invalid displayed PackageId: {}", err))?;

        let source_id = match s.next() {
            None => SourceId::default(),
            Some(source_id) => {
                let source_id = if source_id.starts_with('(') && source_id.ends_with(')') {
                    &source_id[1..source_id.len() - 1]
                } else {
                    bail!(
                        "invalid displayed PackageId: source url is not wrapped with parentheses",
                    );
                };
                SourceId::from_display_str(source_id)?
            }
        };

        Ok(PackageId::new(name, version, source_id))
    }

    pub fn to_serialized_string(&self) -> String {
        format!(
            "{} {} ({})",
            self.name,
            self.version,
            self.source_id.to_pretty_url(),
        )
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
        s.collect_str(&self.to_serialized_string())
    }
}

impl<'de> Deserialize<'de> for PackageId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<PackageId, D::Error> {
        use serde::de::Error;

        let string = String::deserialize(d)?;
        let mut s = string.splitn(3, ' ');

        let name = PackageName::try_new(s.next().unwrap())
            .map_err(|err| Error::custom(format_args!("invalid serialized PackageId: {err}")))?;

        let Some(version) = s.next() else {
            return Err(Error::custom(
                "invalid serialized PackageId: missing version",
            ));
        };
        let version = version
            .to_version()
            .map_err(|err| Error::custom(format_args!("invalid serialized PackageId: {err}")))?;

        let Some(url) = s.next() else {
            return Err(Error::custom(
                "invalid serialized PackageId: missing source url",
            ));
        };
        let url = if url.starts_with('(') && url.ends_with(')') {
            &url[1..url.len() - 1]
        } else {
            return Err(Error::custom(
                "invalid serialized PackageId: source url is not wrapped with parentheses",
            ));
        };
        let source_id = SourceId::from_pretty_url(url).map_err(Error::custom)?;

        Ok(PackageId::new(name, version, source_id))
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
    use semver::Version;
    use serde_test::{assert_de_tokens_error, assert_tokens, Token};
    use test_case::test_case;

    use crate::core::package::PackageId;
    use crate::core::source::SourceId;
    use crate::core::PackageName;

    fn leak_string(string: String) -> &'static str {
        Box::leak(string.into_boxed_str())
    }

    #[test_case(SourceId::mock_git())]
    #[test_case(SourceId::mock_path())]
    fn serialization(source_id: SourceId) {
        let name = PackageName::new("foo");
        let version = Version::new(1, 0, 0);
        let pkg_id = PackageId::new(name, version, source_id);
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
        let name = PackageName::new("foo");
        let version = Version::new(1, 0, 0);
        let source_id = SourceId::mock_path();
        let pkg_id = PackageId::new(name, version, source_id);
        assert_eq!(format!("foo v1.0.0 ({source_id})"), pkg_id.to_string());
    }
}
