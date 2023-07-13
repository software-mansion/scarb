use std::fs;

use anyhow::Result;

pub fn main() -> Result<()> {
    for bin in list_binaries()? {
        println!("{bin}");
    }
    Ok(())
}

pub fn list_binaries() -> Result<Vec<String>> {
    let mut bins = vec!["scarb".to_string()];
    for entry in fs::read_dir("extensions")? {
        let entry = entry?;
        bins.push(entry.file_name().to_string_lossy().to_string());
    }
    Ok(bins)
}
