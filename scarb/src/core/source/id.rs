use std::fmt;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, ensure, Context, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use smol_str::SmolStr;
use url::Url;

use crate::core::source::Source;
use crate::core::Config;
use crate::internal::static_hash_cache::StaticHashCache;

/// Unique identifier for a source of packages.
///
/// See [`SourceIdInner`] for public fields reference.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SourceId(&'static SourceIdInner);

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub struct SourceIdInner {
    /// The source URL.
    pub url: Url,
    /// The source kind.
    pub kind: SourceKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SourceKind {
    /// A local path.
    Path,
    /// A git repository.
    Git(GitReference),
    /// The Cairo core library.
    Core,
}

/// Information to find a specific commit in a Git repository.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum GitReference {
    /// From a tag.
    Tag(SmolStr),
    /// From a branch.
    Branch(SmolStr),
    /// From a specific revision.
    Rev(SmolStr),
    /// The default branch of the repository, the reference named `HEAD`.
    DefaultBranch,
}

impl SourceId {
    fn new(url: Url, kind: SourceKind) -> Result<Self> {
        Ok(Self::pure(url, kind))
    }

    fn pure(url: Url, kind: SourceKind) -> Self {
        static CACHE: StaticHashCache<SourceIdInner> = StaticHashCache::new();
        let inner = SourceIdInner { url, kind };
        Self(CACHE.intern(inner))
    }

    pub fn for_path(path: &Path) -> Result<Self> {
        let url = Url::from_directory_path(path)
            .map_err(|_| anyhow!("path ({}) is not absolute", path.display()))?;
        Self::new(url, SourceKind::Path)
    }

    pub fn for_git(url: &Url, reference: &GitReference) -> Result<Self> {
        Self::new(url.clone(), SourceKind::Git(reference.clone()))
    }

    pub fn for_core() -> Self {
        let url = Url::parse("https://github.com/starkware-libs/cairo.git").unwrap();
        SourceId::pure(url, SourceKind::Core)
    }

    pub fn is_default_registry(self) -> bool {
        // TODO(mkaput): This will be unnecessary when we will have general default registry.
        #[cfg(test)]
        return self == Self::mock_default();

        // TODO(mkaput): Return `true` for default registry here.
        #[cfg(not(test))]
        false
    }

    // TODO(mkaput): This will be unnecessary when we will have general default registry.
    #[cfg(test)]
    pub(crate) fn mock_default() -> Self {
        let url = Url::parse("https://git.test/default.git").unwrap();
        Self::for_git(&url, &GitReference::DefaultBranch).unwrap()
    }

    pub fn is_path(self) -> bool {
        self.kind == SourceKind::Path
    }

    pub fn to_path(self) -> Option<PathBuf> {
        match self.kind {
            SourceKind::Path => Some(
                self.url
                    .to_file_path()
                    .expect("this has to be a file:// URL"),
            ),

            _ => None,
        }
    }

    pub fn is_git(self) -> bool {
        matches!(self.kind, SourceKind::Git(_))
    }

    pub fn to_pretty_url(self) -> String {
        match &self.kind {
            SourceKind::Path => format!("path+{}", self.url),

            SourceKind::Git(reference) => {
                let mut url = self.url.clone();
                match reference {
                    GitReference::Tag(tag) => {
                        url.query_pairs_mut().append_pair("tag", tag);
                    }
                    GitReference::Branch(branch) => {
                        url.query_pairs_mut().append_pair("branch", branch);
                    }
                    GitReference::Rev(rev) => {
                        url.query_pairs_mut().append_pair("rev", rev);
                    }
                    GitReference::DefaultBranch => {}
                }
                format!("git+{}", url)
            }

            SourceKind::Core => format!("core+{}", self.url),
        }
    }

    pub fn from_pretty_url(pretty_url: &str) -> Result<Self> {
        let (kind, url) = {
            let mut parts = pretty_url.splitn(2, '+');
            (
                parts.next().expect("at least one part must be here"),
                parts
                    .next()
                    .ok_or_else(|| anyhow!("invalid source: {pretty_url}"))?,
            )
        };

        let mut url =
            Url::parse(url).with_context(|| format!("cannot parse source URL: {pretty_url}"))?;

        match kind {
            "git" => {
                let mut reference = GitReference::DefaultBranch;
                for (k, v) in url.query_pairs() {
                    match &k[..] {
                        "branch" => reference = GitReference::Branch(v.into()),
                        "rev" => reference = GitReference::Rev(v.into()),
                        "tag" => reference = GitReference::Tag(v.into()),
                        _ => {}
                    }
                }

                url.set_query(None);
                SourceId::for_git(&url, &reference)
            }

            "path" => SourceId::new(url, SourceKind::Path),

            "core" => {
                let stencil = SourceId::for_core();
                ensure!(url == stencil.url, "there is only one valid core url");
                Ok(stencil)
            }

            kind => bail!("unsupported source protocol: {kind}"),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_display_str(string: &str) -> Result<Self> {
        use std::ffi::OsString;
        Self::for_path(&PathBuf::from(OsString::from(string)))
            .or_else(|_| Self::from_pretty_url(string))
    }

    /// Creates an implementation of `Source` corresponding to this ID.
    pub fn load<'c>(self, config: &'c Config) -> Result<Box<dyn Source + 'c>> {
        use crate::sources::*;
        match self.kind {
            SourceKind::Path => Ok(Box::new(PathSource::new(self, config))),
            SourceKind::Git(_) => todo!("Git sources are not implemented yet"),
            SourceKind::Core => Ok(Box::new(CorelibSource::new(config))),
        }
    }
}

#[cfg(test)]
impl SourceId {
    pub(crate) fn mock_git() -> SourceId {
        let url = Url::parse("https://github.com/starkware-libs/cairo.git").unwrap();
        let reference = GitReference::Tag("test".into());
        SourceId::for_git(&url, &reference).unwrap()
    }

    pub(crate) fn mock_path() -> SourceId {
        SourceId::for_path(&std::env::temp_dir()).unwrap()
    }
}

impl Deref for SourceId {
    type Target = SourceIdInner;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl fmt::Debug for SourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SourceId")
            .field(&self.url.to_string())
            .field(&self.kind)
            .finish()
    }
}

impl fmt::Display for SourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.kind == SourceKind::Path {
            let path = self.url.to_file_path().expect("expected file:// URL here");
            write!(f, "{}", path.display())
        } else {
            write!(f, "{}", self.to_pretty_url())
        }
    }
}

impl Serialize for SourceId {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(&self.to_pretty_url())
    }
}

impl<'de> Deserialize<'de> for SourceId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<SourceId, D::Error> {
        use serde::de::Error;
        let string = String::deserialize(d)?;
        SourceId::from_pretty_url(&string).map_err(Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use crate::core::source::SourceId;

    #[test_case(SourceId::mock_git())]
    #[test_case(SourceId::mock_path())]
    #[test_case(SourceId::for_core())]
    fn equality_after_pretty_url_conversion(source_id: SourceId) {
        assert_eq!(
            SourceId::from_pretty_url(&source_id.to_pretty_url()).unwrap(),
            source_id
        );
    }
}
