use anyhow::{ensure, Result};
use clap::Parser;
use semver::Version;
use toml_edit::{value, Document};
use xshell::{cmd, Shell};

pub fn expected_scarb_version() -> Result<Version> {
    // NOTE: We are reading lockfile manually here, so that we are not dependent on when this
    // program was built (that would be the case when using scarb-build-metadata). We are also
    // deliberately not using cargo_metadata, to reduce build times of xtasks.

    let sh = Shell::new()?;
    let cargo_lock = sh.read_file("Cargo.lock")?.parse::<Document>()?;
    let packages = cargo_lock["package"].as_array_of_tables().unwrap();
    let compiler = {
        let pkgs = packages
            .into_iter()
            .filter(|pkg| pkg["name"].as_str().unwrap() == "cairo-lang-compiler")
            .collect::<Vec<_>>();
        ensure!(
            pkgs.len() == 1,
            "expected exactly one cairo-lang-compiler package in Cargo.lock, found: {}",
            pkgs.len()
        );
        pkgs.into_iter().next().unwrap()
    };
    let compiler_version = compiler["version"].as_str().unwrap();
    Ok(compiler_version.parse()?)
}

#[derive(Default, Parser)]
pub struct Args {
    #[arg(long)]
    pub build: Option<String>,
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub fn main(args: Args) -> Result<()> {
    let sh = Shell::new()?;

    let mut cargo_toml = sh.read_file("Cargo.toml")?.parse::<Document>()?;
    let package = cargo_toml["workspace"]["package"].as_table_mut().unwrap();

    let mut version = expected_scarb_version()?;

    if let Some(build) = args.build {
        version.build = build.parse()?;
    }

    package["version"] = value(version.to_string());

    eprintln!("[workspace.package]\n{package}");

    if !args.dry_run {
        sh.write_file("Cargo.toml", cargo_toml.to_string())?;

        cmd!(sh, "cargo fetch").run()?;
    }

    Ok(())
}
