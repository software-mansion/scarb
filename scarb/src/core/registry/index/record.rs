use crate::core::{Checksum, PackageName};
use dialoguer::console::Style;
use scarb_ui::Message;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

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
    #[serde(default = "default_false", skip_serializing_if = "is_false")]
    pub yanked: bool,
    #[serde(default = "default_false", skip_serializing_if = "is_false")]
    pub audited: bool,
}

// TODO: replace with impl for IndexRecords with proper formatting for yank / audit status
impl Message for IndexRecord {
    fn text(self) -> String {
        if self.yanked {
            Style::from_dotted_str("red")
                .apply_to(format!("{} (yanked)", self.version))
                .to_string()
        } else {
            format!("{}", self.version)
        }
    }
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
