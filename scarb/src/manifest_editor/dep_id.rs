use std::fmt;
use std::str::FromStr;

use semver::VersionReq;

use crate::core::PackageName;

/// Reference to a package to be added as a dependency.
///
/// See `scarb add` help for more info.
#[derive(Clone, Debug, Default)]
pub struct DepId {
    pub name: Option<PackageName>,
    pub version_req: Option<VersionReq>,
}

impl DepId {
    pub fn unspecified() -> Self {
        Self::default()
    }
}

impl FromStr for DepId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let mut dep = DepId::default();

        if s.is_empty() {
            return Ok(dep);
        }

        let mut s = s.split('@');
        let Some(name) = s.next() else {
            return Ok(dep);
        };
        dep.name = Some(name.parse()?);

        let Some(version_req) = s.next() else {
            return Ok(dep);
        };
        dep.version_req = Some(version_req.parse()?);

        Ok(dep)
    }
}

impl fmt::Display for DepId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = &self.name {
            write!(f, "{name}")?;
        }

        if let Some(version_req) = &self.version_req {
            write!(f, "@{version_req}")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::DepId;

    #[test_case("", None, None)]
    #[test_case("abc", Some("abc"), None)]
    #[test_case("abc@1", Some("abc"), Some("^1"))]
    fn dep_is_from_str(s: &str, expected_name: Option<&str>, expected_version: Option<&str>) {
        let dep: DepId = s.parse().expect("parsing dep id failed");
        assert_eq!(
            (
                dep.name.map(|p| p.to_string()),
                dep.version_req.map(|p| p.to_string())
            ),
            (
                expected_name.map(|n| n.to_string()),
                expected_version.map(|v| v.to_string())
            ),
        );
    }
}
