use anyhow::Result;
use cairo_toolchain_xtasks::sync_version::expected_version;
use clap::Parser;
use semver::{BuildMetadata, Prerelease, Version};
use time::OffsetDateTime;

#[derive(Parser)]
pub struct Args {
    #[arg(short, long)]
    tag: bool,
    #[arg(long, env = "IS_SCARB_DEV")]
    dev: bool,
}

pub fn main(args: Args) -> Result<()> {
    let tag = nightly_tag(args.dev);
    if args.tag {
        println!("{tag}");
    } else {
        let version = nightly_version(args.dev)?;
        println!("{version}");
    }
    Ok(())
}

pub fn nightly_version(is_dev: bool) -> Result<Version> {
    let mut version = expected_version()?;
    version.pre = Prerelease::EMPTY;
    version.build = BuildMetadata::new(&nightly_tag(is_dev)).unwrap();
    Ok(version)
}

pub fn nightly_tag(is_dev: bool) -> String {
    let prefix = if is_dev { "dev" } else { "nightly" };
    let dt = OffsetDateTime::now_utc();
    format!(
        "{prefix}-{}-{:0>2}-{:0>2}",
        dt.year(),
        u8::from(dt.month()),
        dt.day()
    )
}
