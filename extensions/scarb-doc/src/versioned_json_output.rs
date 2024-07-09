use crate::types::Crate;
use serde::Serialize;
use std::collections::BTreeMap;

type PackageName = String;

const FORMAT_VERSION: u8 = 1;

#[derive(Serialize)]
pub struct VersionedJsonOutput {
    format_version: u8,
    pub package_information_map: BTreeMap<PackageName, Crate>,
}

impl VersionedJsonOutput {
    pub fn new(package_information_map: BTreeMap<PackageName, Crate>) -> Self {
        Self {
            format_version: FORMAT_VERSION,
            package_information_map,
        }
    }
}
