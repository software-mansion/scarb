use std::fmt;

use id_arena::Id;
use semver::Version;

use crate::resolver::package_range::PackageRange;
use crate::resolver::package_ref::PackageRef;
use crate::resolver::term::Term;
use crate::resolver::version_constraint::VersionConstraint;

pub type IncompatibilityId = Id<Incompatibility>;

#[derive(Clone, Debug)]
pub struct Incompatibility {
    terms: Vec<Term>,
    cause: Cause,
}

#[derive(Clone, Debug)]
enum Cause {
    Root,
    Dependency,
    NoVersions,
    PackageNotFound,
    Conflict(IncompatibilityId, IncompatibilityId),
}

impl Incompatibility {
    pub fn root() -> Self {
        Self {
            terms: vec![Term::positive(PackageRange::without_source(
                PackageRef::Root,
                VersionConstraint::exact(Version::new(1, 0, 0)),
            ))],
            cause: Cause::Root,
        }
    }

    pub fn terms(&self) -> impl Iterator<Item = &Term> + '_ {
        self.terms.iter()
    }
}

impl fmt::Display for Incompatibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}
