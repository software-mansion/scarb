use crate::{
    errors::{IODirectoryCreationError, IOWriteError, PackagesSerializationError},
    PackageInformation,
};
use anyhow::Result;
use camino::Utf8Path;
use serde::Serialize;
use std::fs;

const FORMAT_VERSION: u8 = 1;
const JSON_OUTPUT_FILENAME: &str = "output.json";

#[derive(Serialize)]
pub struct VersionedJsonOutput {
    format_version: u8,
    packages_information: Vec<PackageInformation>,
}

impl VersionedJsonOutput {
    pub fn new(packages_information: Vec<PackageInformation>) -> Self {
        Self {
            format_version: FORMAT_VERSION,
            packages_information,
        }
    }

    pub fn save_to_file(&self, output_dir: &Utf8Path) -> Result<()> {
        fs::create_dir_all(output_dir)
            .map_err(|e| IODirectoryCreationError::new(e, "generated documentation"))?;

        let output_path = output_dir.join(JSON_OUTPUT_FILENAME);

        let output =
            serde_json::to_string_pretty(&self).map_err(PackagesSerializationError::from)? + "\n";

        fs::write(output_path, output).map_err(|e| IOWriteError::new(e, "json documentation"))?;

        Ok(())
    }
}
