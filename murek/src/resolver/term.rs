use std::fmt;

use crate::resolver::package_range::PackageRange;

#[derive(Clone, Debug)]
pub struct Term {
    pub positive: bool,
    pub package_range: PackageRange,
}

impl Term {
    pub fn positive(package_range: PackageRange) -> Self {
        Self {
            positive: true,
            package_range,
        }
    }

    pub fn negative(package_range: PackageRange) -> Self {
        Self {
            positive: false,
            package_range,
        }
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.positive {
            write!(f, "not ")?;
        }
        write!(f, "{}", self.package_range)?;
        Ok(())
    }
}
