use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use semver::{BuildMetadata, Prerelease, Version};

#[derive(Parser)]
pub struct Args {
    #[arg(short, long)]
    tag: bool,
}

pub fn main(args: Args) -> Result<()> {
    let tag = nightly_tag();
    if args.tag {
        println!("{tag}");
    } else {
        let version = nightly_version();
        println!("{version}");
    }
    Ok(())
}

pub fn nightly_version() -> Version {
    // NOTE: We are not using scarb-build-metadata here to reduce compilation times of xtask crate.
    let mut version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
    version.pre = Prerelease::EMPTY;
    version.build = BuildMetadata::new(&nightly_tag()).unwrap();
    version
}

pub fn nightly_tag() -> String {
    let dt = Utc::now();
    format!("nightly-{}", dt.format("%Y-%m-%d"))
}
