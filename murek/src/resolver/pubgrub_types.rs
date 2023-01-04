use std::fmt;
use std::fmt::Debug;

use pubgrub::range::Range;
use semver::Op;
use serde::{Deserialize, Serialize};

use crate::core::package::PackageName;
use crate::core::{PackageId, SourceId};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub struct PubGrubPackage {
    pub name: PackageName,
}

impl PubGrubPackage {
    pub fn new(name: PackageName) -> Self {
        Self { name }
    }
}

impl fmt::Display for PubGrubPackage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.name, f)
    }
}

impl From<PackageId> for PubGrubPackage {
    fn from(id: PackageId) -> Self {
        PubGrubPackage::new(id.name.clone())
    }
}

impl From<PackageName> for PubGrubPackage {
    fn from(name: PackageName) -> Self {
        PubGrubPackage::new(name)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PubGrubVersion {
    pub version: semver::Version,
    pub source_id: SourceId,
}

impl PubGrubVersion {
    pub fn new(version: semver::Version, source_id: SourceId) -> Self {
        if !version.pre.is_empty() || !version.build.is_empty() {
            todo!("Prerelease and build metadata parts in versions are not supported yet by version solver.");
        }
        Self { version, source_id }
    }

    pub fn with_default_source(version: semver::Version) -> Self {
        Self::new(version, Default::default())
    }

    pub fn as_package_id(&self, name: &PackageName) -> PackageId {
        PackageId::pure(name.clone(), self.version.clone(), self.source_id)
    }
}

impl Default for PubGrubVersion {
    fn default() -> Self {
        PubGrubVersion::with_default_source(semver::Version::new(0, 0, 0))
    }
}

impl fmt::Display for PubGrubVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.version)?;

        if !self.source_id.is_main_registry() {
            write!(f, " ({})", self.source_id)?;
        }

        Ok(())
    }
}

impl pubgrub::version::Version for PubGrubVersion {
    fn lowest() -> Self {
        Default::default()
    }

    fn bump(&self) -> Self {
        let mut v = self.clone();
        if v.version.pre.is_empty() && v.version.build.is_empty() {
            v.version.patch += 1;
        } else {
            v.version.pre = semver::Prerelease::EMPTY;
            v.version.build = semver::BuildMetadata::EMPTY;
        }
        v
    }
}

impl From<PackageId> for PubGrubVersion {
    fn from(id: PackageId) -> Self {
        Self {
            version: id.version.clone(),
            source_id: id.source_id,
        }
    }
}

pub fn package_id_from_pubgrub(p: &PubGrubPackage, v: &PubGrubVersion) -> PackageId {
    PackageId::pure(p.name.clone(), v.version.clone(), v.source_id)
}

pub fn pubgrub_range_from_version_req_and_source_id(
    req: semver::VersionReq,
    source_id: SourceId,
) -> Range<PubGrubVersion> {
    fn range_from_comparator(
        comparator: semver::Comparator,
        source_id: SourceId,
    ) -> Range<PubGrubVersion> {
        let pivot = PubGrubVersion::new(
            semver::Version {
                major: comparator.major,
                // TODO(mkaput): Implement wildcards.
                minor: comparator.minor.expect("Version requirements with `minor` part omitted are not supported yet by version solver."),
                patch: comparator.patch.expect("Version requirements with `patch` part omitted are not supported yet by version solver."),
                pre: comparator.pre,
                build: Default::default()
            },
            source_id
        );

        match comparator.op {
            Op::Exact => Range::exact(pivot),
            Op::GreaterEq => Range::higher_than(pivot),
            op @ (Op::Greater | Op::Less | Op::LessEq | Op::Tilde | Op::Caret | Op::Wildcard) => {
                todo!(
                    "\
                Wildcards and operators >, <, <=, ~ and ^ in version requirements \
                are not supported yet by version solver. \
                You used {op:?}. \
                Use exact matches or >= only."
                )
            }
            op => unimplemented!("Unknown version requirement operator {op:?}."),
        }
    }

    req.comparators
        .into_iter()
        .fold(Range::any(), |range, comparator| {
            range.intersection(&range_from_comparator(comparator, source_id))
        })
}

pub fn version_req_and_source_id_from_pubgrub_range(
    _range: &Range<PubGrubVersion>,
) -> (semver::VersionReq, SourceId) {
    todo!()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::str::FromStr;

    use itertools::iproduct;
    use pubgrub::version::Version;
    use semver::{BuildMetadata, Prerelease, VersionReq};
    use test_case::test_case;

    use crate::core::SourceId;

    use super::PubGrubVersion;

    #[test_case("1.0.0" => "1.0.1")]
    #[test_case("0.1.1" => "0.1.2")]
    // TODO(mkaput): Support prerelease and build metadata.
    // #[test_case("1.0.0-alpha" => "1.0.0")]
    // #[test_case("1.0.0-rc.1" => "1.0.0")]
    fn version_bump(input: &str) -> String {
        let version =
            PubGrubVersion::with_default_source(semver::Version::from_str(input).unwrap());
        version.bump().to_string()
    }

    #[test]
    fn version_bump_with_custom_source() {
        let source_id = SourceId::for_path(&PathBuf::from("/foo/bar")).unwrap();
        let version = PubGrubVersion::new(semver::Version::new(1, 0, 0), source_id);
        assert_eq!(
            version.bump(),
            PubGrubVersion::new(semver::Version::new(1, 0, 1), source_id)
        )
    }

    #[test_case("=1.0.0", "1.0.0 (/path/)")]
    #[test_case(">=1.0.0", "1.0.0 (/path/) <= v")]
    #[test_case("*", "∗")]
    // TODO(mkaput): Support other operators.
    // #[test_case("1.0.0", "1.0.0 (/path/) <= v < 2.0.0 (/path/)")]
    fn pubgrub_version_range_from_version_req_and_source_id(
        version_req_str: &str,
        expected_range_str: &str,
    ) {
        let source_id = SourceId::for_path(&PathBuf::from("/path")).unwrap();
        let version_req = VersionReq::from_str(version_req_str).unwrap();
        let range =
            super::pubgrub_range_from_version_req_and_source_id(version_req.clone(), source_id);
        assert_eq!(expected_range_str, range.to_string());

        let mut expected_matches = BTreeMap::new();
        let mut actual_matches = BTreeMap::new();
        for (major, minor, patch, pre_opt, build_opt) in iproduct!(
            0..=4,
            0..=4,
            0..=4,
            // TODO(mkaput): Support prerelease and build metadata.
            [None /*Some("alpha")*/,],
            [None /*Some("0")*/,]
        ) {
            let example_version = semver::Version {
                major,
                minor,
                patch,
                pre: pre_opt
                    .map(|text| Prerelease::from_str(text).unwrap())
                    .unwrap_or_default(),
                build: build_opt
                    .map(|text| BuildMetadata::from_str(text).unwrap())
                    .unwrap_or_default(),
            };

            let example_pubgrub_version = PubGrubVersion::new(example_version.clone(), source_id);

            let expected = version_req.matches(&example_version);
            let actual = range.contains(&example_pubgrub_version);
            if expected != actual {
                expected_matches.insert(example_version.to_string(), expected);
                actual_matches.insert(example_version.to_string(), actual);
            }
        }
        assert_eq!(
            expected_matches, actual_matches,
            "Range does not match versions as expected, \
            version_req: {version_req_str}, \
            range: {expected_range_str}, \
            listing differences where lefts are expected and rights are actual."
        );
    }
}
