use anyhow::Result;
use clap::Parser;
use semver::Version;
use toml_edit::{value, Document};
use xshell::{cmd, Shell};

#[derive(Parser)]
pub struct Args {
    version: Version,
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

pub fn main(args: Args) -> Result<()> {
    let sh = Shell::new()?;

    let mut cargo_toml = sh.read_file("Cargo.toml")?.parse::<Document>()?;
    let package = cargo_toml["workspace"]["package"].as_table_mut().unwrap();

    package["version"] = value(args.version.to_string());

    eprintln!("[workspace.package]\n{package}");

    if !args.dry_run {
        sh.write_file("Cargo.toml", cargo_toml.to_string())?;

        cmd!(sh, "cargo fetch").run()?;
    }

    Ok(())
}
