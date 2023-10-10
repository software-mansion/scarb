use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

use crate::core::{Checksum, PackageName};

pub type IndexRecords = Vec<IndexRecord>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IndexRecord {
    #[serde(rename = "v")]
    pub version: Version,
    #[serde(rename = "deps")]
    pub dependencies: IndexDependencies,
    #[serde(rename = "cksum")]
    pub checksum: Checksum,
    #[serde(default = "default_false", skip_serializing_if = "is_false")]
    pub no_core: bool,
}

pub type IndexDependencies = Vec<IndexDependency>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IndexDependency {
    pub name: PackageName,
    pub req: VersionReq,
}

fn default_false() -> bool {
    false
}

fn is_false(value: &bool) -> bool {
    !*value
}
