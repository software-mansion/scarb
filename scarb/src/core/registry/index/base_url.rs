use std::ops::Deref;
use std::str::FromStr;

use anyhow::ensure;
use serde::{Deserialize, Serialize};
use url::Url;

/// Wrapper over [`Url`] which ensures that the URL can be a joined, and has trailing slash.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Url")]
pub struct BaseUrl(Url);

impl Deref for BaseUrl {
    type Target = Url;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<Url> for BaseUrl {
    type Error = anyhow::Error;

    fn try_from(mut url: Url) -> Result<Self, Self::Error> {
        ensure!(!url.cannot_be_a_base(), "invalid base url: {url}");

        if !url.path().ends_with('/') {
            url.set_path(&format!("{}/", url.path()));
        }

        Ok(Self(url))
    }
}

impl FromStr for BaseUrl {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        BaseUrl::try_from(Url::parse(s)?)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use test_case::test_case;

    use super::BaseUrl;

    #[test]
    fn rejects_cannot_be_a_base_urls() {
        assert_eq!(
            "invalid base url: data:text/plain,Stuff",
            BaseUrl::from_str("data:text/plain,Stuff")
                .unwrap_err()
                .to_string(),
        );
    }

    #[test_case("https://example.com" => "https://example.com/")]
    #[test_case("https://example.com/file" => "https://example.com/file/")]
    #[test_case("https://example.com/path/" => "https://example.com/path/")]
    #[test_case("https://example.com/file.json" => "https://example.com/file.json/")]
    fn appends_trailing_slash_if_missing(url: &str) -> String {
        BaseUrl::from_str(url).unwrap().to_string()
    }
}
