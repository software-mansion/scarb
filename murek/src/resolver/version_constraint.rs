use std::fmt;

use semver::{Comparator, Op, Version, VersionReq};

#[derive(Clone, Debug)]
pub struct VersionConstraint {
    req: VersionReq,
}

impl VersionConstraint {
    pub fn exact(version: Version) -> Self {
        Self {
            req: VersionReq::from_iter([Comparator {
                op: Op::Exact,
                major: version.major,
                minor: Some(version.minor),
                patch: Some(version.patch),
                pre: version.pre,
            }]),
        }
    }

    pub fn from_req(req: VersionReq) -> Self {
        Self { req }
    }

    pub fn is_all(&self) -> bool {
        self.req.comparators.is_empty()
    }
}

impl From<Version> for VersionConstraint {
    fn from(version: Version) -> Self {
        VersionConstraint::exact(version)
    }
}

impl From<VersionReq> for VersionConstraint {
    fn from(req: VersionReq) -> Self {
        VersionConstraint::from_req(req)
    }
}

impl fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.req)
    }
}
