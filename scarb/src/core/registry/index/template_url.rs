use std::fmt;

use anyhow::{bail, Context, Result};
use indoc::formatdoc;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::core::{PackageId, PackageName};

/// A template string which will generate the download or index URL for the specific package.
///
/// The patterns `{package}` and `{version}` will be replaced with the package name and version
/// (without leading `v`) respectively. The pattern `{prefix}` will be replaced with the package's
/// prefix directory name (e.g. `ab/cd` for package named `abcd`).
///
/// If the template contains no patterns, the template will expand to itself, which probably is
/// not something you want.
///
/// Upon expansion, the template must form a valid URL.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TemplateUrl(String);

#[derive(Default)]
pub struct ExpansionParams {
    pub package: Option<String>,
    pub version: Option<String>,
}

impl TemplateUrl {
    pub fn new(template: &str) -> Self {
        Self(template.to_owned())
    }

    pub fn expand(&self, params: ExpansionParams) -> Result<Url> {
        let prefix = params.package.as_deref().map(pkg_prefix);

        let replace = |s: &mut String, pattern: &str, expansion: Option<String>| -> Result<()> {
            match expansion {
                Some(expansion) => {
                    *s = s.replace(pattern, &expansion);
                    Ok(())
                }
                None if s.contains(pattern) => bail!(
                    "pattern `{pattern}` in not available in this context for template url: {self}"
                ),
                None => Ok(()),
            }
        };

        let mut expansion = self.0.clone();
        replace(&mut expansion, "{package}", params.package)?;
        replace(&mut expansion, "{version}", params.version)?;
        replace(&mut expansion, "{prefix}", prefix)?;

        expansion.parse().with_context(|| {
            formatdoc! {r"
                failed to expand template url:
                template:  {self}
                expansion: {expansion}
            "}
        })
    }
}

impl fmt::Display for TemplateUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl From<PackageName> for ExpansionParams {
    fn from(package: PackageName) -> Self {
        (&package).into()
    }
}

impl From<&PackageName> for ExpansionParams {
    fn from(package: &PackageName) -> Self {
        Self {
            package: Some(package.to_string()),
            ..Default::default()
        }
    }
}

impl From<PackageId> for ExpansionParams {
    fn from(package: PackageId) -> Self {
        Self {
            package: Some(package.name.to_string()),
            version: Some(package.version.to_string()),
        }
    }
}

/// Make a path to a package directory, which aligns to the index directory layout.
fn pkg_prefix(name: &str) -> String {
    match name.len() {
        1 => "1".to_string(),
        2 => "2".to_string(),
        3 => format!("3/{}", &name[..1]),
        _ => format!("{}/{}", &name[0..2], &name[2..4]),
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use crate::core::{PackageId, PackageName};

    use super::TemplateUrl;

    #[test]
    fn expand() {
        let template = TemplateUrl::new("https://example.com/{prefix}/{package}-{version}.json");
        let package_id = PackageId::from_display_str("foobar v1.0.0").unwrap();
        assert_eq!(
            "https://example.com/fo/ob/foobar-1.0.0.json",
            template.expand(package_id.into()).unwrap().as_str()
        )
    }

    #[test]
    fn expand_missing_pattern() {
        let template = TemplateUrl::new("https://example.com/{prefix}/{package}-{version}.json");
        assert_eq!(
            template
                .expand(PackageName::CORE.into())
                .unwrap_err()
                .to_string(),
            "pattern `{version}` in not available in this context for template url: \
            https://example.com/{prefix}/{package}-{version}.json"
        );
    }

    #[test_case("a" => "1")]
    #[test_case("ab" => "2")]
    #[test_case("abc" => "3/a")]
    #[test_case("Xyz" => "3/X")]
    #[test_case("AbCd" => "Ab/Cd")]
    #[test_case("pQrS" => "pQ/rS")]
    fn pkg_prefix(input: &str) -> String {
        super::pkg_prefix(input)
    }
}
