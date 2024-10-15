use anyhow::Result;
use clap::Parser;
use xshell::{cmd, Shell};
use serde_json::Value;

#[derive(Parser)]
pub struct Args;

pub fn main(_args: Args) -> Result<()> {
    let sh = Shell::new()?;

    let cargo_metadata = cmd!(sh, "cargo metadata -q --format-version 1").read()?;
    let cargo_metadata: Value = serde_json::from_str(&cargo_metadata)?;

    let cairo_compiler_package_id = get_cairo_compiler_package_id(&cargo_metadata);
    let cairo_version = get_cairo_version(&cargo_metadata, &cairo_compiler_package_id);

    println!("{cairo_version}");

    Ok(())
}

pub fn get_cairo_compiler_package_id(cargo_metadata: &Value) -> String {
    let cairo_compiler_package_id = cargo_metadata
        .get("resolve")
        .unwrap()
        .get("nodes")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|node| {
            let repr = node.get("id")
                .unwrap()
                .as_str()
                .unwrap();
            // The first condition for Rust >= 1.77
            // (After the PackageId spec stabilization)
            // The second condition for Rust < 1.77
            repr.contains("scarb#") || repr.starts_with("scarb ")
        })
        .unwrap()
        .get("deps")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|dep| dep.get("name").unwrap() == "cairo_lang_compiler")
        .unwrap()
        .get("pkg")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();

    cairo_compiler_package_id
}

pub fn get_cairo_version(cargo_metadata: &Value, cairo_compiler_package_id: &str) -> String {
    let cairo_package = cargo_metadata
        .get("packages")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|pkg| pkg.get("id").unwrap() == cairo_compiler_package_id)
        .unwrap();

    let cairo_version = cairo_package.get("version").unwrap().as_str().unwrap().to_string();

    cairo_version
}