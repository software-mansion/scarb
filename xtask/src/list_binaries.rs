use anyhow::Result;
use std::fs;

pub fn main() -> Result<()> {
    println!("scarb");
    for entry in fs::read_dir("extensions")? {
        let entry = entry?;
        println!("{}", entry.file_name().to_string_lossy());
    }
    Ok(())
}
