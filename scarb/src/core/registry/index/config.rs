use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::core::registry::index::{BaseUrl, TemplateUrl};

/// The `config.json` file stored in and defining the index.
///
/// The config file may look like this:
///
/// ```json
/// {
///   "version": 1,
///   "api": "https://example.com/api/v1",
///   "dl": "https://example.com/api/v1/download/{package}/{version}",
///   "upload": "https://example.com/api/v1/packages/new",
///   "index": "https://example.com/index/{prefix}/{package}.json"
/// }
/// ```
///
/// ## URL Templates
///
/// The values for the `"dl"` and `"index"` fields are URL templates.
/// See documentation for [`TemplateUrl`] for supported expansion patterns.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct IndexConfig {
    /// Index version, must be `1` (numeric).
    pub version: IndexVersion,

    /// API endpoint for the registry.
    ///
    /// This is what's actually hit to perform operations like yanks, owner modifications,
    /// publish new packages, etc.
    /// If this is `None`, the registry does not support API commands.
    pub api: Option<BaseUrl>,

    /// Download endpoint for all packages.
    pub dl: TemplateUrl,

    /// Base URL for main index file files.
    ///
    /// Usually, this is a location where `config.json` lies, as the rest of index files resides
    /// alongside config.
    pub index: TemplateUrl,

    /// Upload endpoint for all packages.
    ///
    /// If this is `None`, the registry does not support package uploads.
    pub upload: Option<Url>,
}

impl IndexConfig {
    pub const WELL_KNOWN_PATH: &'static str = "api/v1/index/config.json";
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(into = "u8", try_from = "u8")]
pub struct IndexVersion;

impl TryFrom<u8> for IndexVersion {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        ensure!(value == 1, "unsupported index version: {value}");
        Ok(Self)
    }
}

impl From<IndexVersion> for u8 {
    fn from(_: IndexVersion) -> Self {
        1
    }
}

#[cfg(test)]
mod tests {
    use crate::core::registry::index::TemplateUrl;

    use super::IndexConfig;

    #[test]
    fn deserialize() {
        let expected = IndexConfig {
            version: Default::default(),
            api: Some("https://example.com/api/v1/".parse().unwrap()),
            upload: Some("https://example.com/api/v1/packages/new".parse().unwrap()),
            dl: TemplateUrl::new("https://example.com/api/v1/download/{package}/{version}"),
            index: TemplateUrl::new("https://example.com/index/{prefix}/{package}.json"),
        };

        let actual: IndexConfig = serde_json::from_str(
            r#"{
              "version": 1,
              "api": "https://example.com/api/v1",
              "upload": "https://example.com/api/v1/packages/new",
              "dl": "https://example.com/api/v1/download/{package}/{version}",
              "index": "https://example.com/index/{prefix}/{package}.json"
            }"#,
        )
        .unwrap();

        assert_eq!(actual, expected);
    }
}
