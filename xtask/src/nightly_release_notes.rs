use anyhow::Result;
use clap::Parser;
use xshell::{cmd, Shell};

use crate::get_nightly_version::{nightly_tag, nightly_version};

#[derive(Parser)]
pub struct Args;

pub fn main(_: Args) -> Result<()> {
    // Note: We do not use scarb-build-metadata here because it requires rebuilding xtasks.

    let sh = Shell::new()?;

    let tag = nightly_tag();
    let version = nightly_version();

    let scarb_commit = cmd!(sh, "git log -1 --date=short --format=%H").read()?;

    let cargo_metadata = cmd!(sh, "cargo metadata -q --format-version 1").read()?;
    let cargo_metadata = serde_json::from_str::<serde_json::Value>(&cargo_metadata)?;

    let cairo_compiler_package_id = cargo_metadata
        .get("resolve")
        .unwrap()
        .get("nodes")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|node| {
            node.get("id")
                .unwrap()
                .as_str()
                .unwrap()
                .starts_with("scarb ")
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
        .unwrap();

    let cairo_package = cargo_metadata
        .get("packages")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|pkg| pkg.get("id").unwrap() == cairo_compiler_package_id)
        .unwrap();
    let cairo_version = cairo_package.get("version").unwrap().as_str().unwrap();
    let cairo_commit = commit_from_source(cairo_package.get("source").unwrap().as_str().unwrap());

    let scarb_sourced_from = sourced_from("software-mansion/scarb", Some(&scarb_commit));
    let cairo_sourced_from = sourced_from("starkware-libs/cairo", cairo_commit);

    println!(
        "\
# Scarb {tag}

* Scarb `{version}`{scarb_sourced_from}
* Cairo `{cairo_version}`{cairo_sourced_from}
"
    );

    Ok(())
}

fn sourced_from(repo: &str, hash: Option<&str>) -> String {
    let Some(hash) = hash else {
        return String::new();
    };
    let short = shorten(hash);
    format!(" sourced from [`{short}`](https://github.com/{repo}/commit/{hash})")
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
