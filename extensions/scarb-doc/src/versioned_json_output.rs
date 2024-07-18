use crate::PackageInformation;
use anyhow::{Context, Result};
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
            .context("failed to create output directory for scarb doc")?;

        let output_path = output_dir.join(JSON_OUTPUT_FILENAME);

        let output = serde_json::to_string_pretty(&self)
            .expect("failed to serialize information about crates")
            + "\n";

        fs::write(output_path, output)?;

        Ok(())
    }
}
