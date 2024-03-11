use std::fs;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
pub struct Args;

pub fn main(_: Args) -> Result<()> {
    for bin in list_binaries()? {
        println!("{bin}");
    }
    Ok(())
}

/// List all the binaries that should be compiled with Scarb release.
///
/// This includes both the Scarb itself, and all the extensions defined in creates
/// in the `extensions` directory.
///
/// Note that some external dependencies might still define this list statically in their build scripts.
/// One example would be the Homebrew installation formula for scarb.
/// See: `<https://github.com/Homebrew/homebrew-core/blob/master/Formula/s/scarb.rb#L26>`
/// Such dependencies need to be updated manually each time the list of binaries changes.
pub fn list_binaries() -> Result<Vec<String>> {
    let mut bins = vec!["scarb".to_string()];
    for entry in fs::read_dir("extensions")? {
        let entry = entry?;
        bins.push(entry.file_name().to_string_lossy().to_string());
    }
    Ok(bins)
}
