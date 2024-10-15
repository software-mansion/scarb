use anyhow::Result;
use clap::Parser;
use xshell::{cmd, Shell};
use crate::get_cairo_version::{get_cairo_version, get_cairo_compiler_package_id};
use crate::get_nightly_version::nightly_version;

#[derive(Parser)]
pub struct Args;

pub fn main(_: Args) -> Result<()> {
    // Note: We do not use scarb-build-metadata here because it requires rebuilding xtasks.

    let sh = Shell::new()?;

    let version = nightly_version()?;

    let scarb_commit = cmd!(sh, "git log -1 --date=short --format=%H").read()?;

    let cargo_metadata = cmd!(sh, "cargo metadata -q --format-version 1").read()?;
    let cargo_metadata = serde_json::from_str::<serde_json::Value>(&cargo_metadata)?;

    let cairo_compiler_package_id = get_cairo_compiler_package_id(&cargo_metadata);
    let cairo_package = cargo_metadata
        .get("packages")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|pkg| pkg.get("id").unwrap() == cairo_compiler_package_id.as_str())
        .unwrap();
    let cairo_version = get_cairo_version(&cargo_metadata, &cairo_compiler_package_id);
    let cairo_commit = commit_from_source(cairo_package.get("source").unwrap().as_str().unwrap());

    let scarb_source_commit = source_commit("software-mansion/scarb", Some(&scarb_commit));
    let cairo_source_commit = source_commit("starkware-libs/cairo", cairo_commit);

    println!(
        "\
| Component | Version           | Source commit         |
|-----------|-------------------|-----------------------|
| Scarb     | `{version}`       | {scarb_source_commit} |
| Cairo     | `{cairo_version}` | {cairo_source_commit} |
"
    );

    Ok(())
}

fn source_commit(repo: &str, hash: Option<&str>) -> String {
    let Some(hash) = hash else {
        return String::new();
    };
    let short = shorten(hash);
    format!("[`{short}`](https://github.com/{repo}/commit/{hash})")
}

fn shorten(hash: &str) -> String {
    let mut s = hash.to_owned();
    s.truncate(9);
    s
}

fn commit_from_source(source: &str) -> Option<&str> {
    if !source.starts_with("git+") {
        return None;
    }

    source.rsplit_once('#').map(|(_, commit)| commit)
}
