use crate::PackageInformation;
use serde::Serialize;

const FORMAT_VERSION: u8 = 1;

#[derive(Serialize)]
pub struct VersionedJsonOutput {
    format_version: u8,
    pub packages_information: Vec<PackageInformation>,
}

impl VersionedJsonOutput {
    pub fn new(packages_information: Vec<PackageInformation>) -> Self {
        Self {
            format_version: FORMAT_VERSION,
            packages_information,
        }
    }
}
