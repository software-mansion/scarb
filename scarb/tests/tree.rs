

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_tree_basic() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let temp_path = temp_dir.path();

    // Create a simple project with dependencies
    std::fs::create_dir_all(temp_path.join("src"))?;
    std::fs::write(
        temp_path.join("Scarb.toml"),
        r#"
[package]
name = "test_project"
version = "0.1.0"

[dependencies]
starknet = "2.0.0"
        "#,
    )?;
    std::fs::write(temp_path.join("src/lib.cairo"), "fn main() {}")?;

    // Run the tree command
    let output = Command::cargo_bin("scarb")?
        .current_dir(temp_path)
        .arg("tree")
        .output()?;

    // Verify the output contains the expected dependency tree
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("test_project v0.1.0"));
    assert!(stdout.contains("starknet"));

    Ok(())
}