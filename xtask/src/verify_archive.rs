use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, ensure};
use clap::Parser;
use walkdir::WalkDir;
use xshell::{Shell, cmd};

#[derive(Parser)]
pub struct Args {
    #[arg(short, long, env = "SCARB_ARCHIVE")]
    archive: PathBuf,
    #[arg(short, long, env = "EXPECTED_VERSION")]
    expected_version: String,
    // Ensure Scarb has been compiled with all optional components.
    #[arg(long)]
    full: bool,
}

pub fn main(args: Args) -> Result<()> {
    let sh = Shell::new()?;

    let expected_version = args.expected_version.trim_start_matches('v');

    let install_dir = sh.create_temp_dir()?;
    if args
        .archive
        .file_name()
        .unwrap()
        .to_string_lossy()
        .ends_with(".tar.gz")
    {
        let archive = &args.archive;
        let install_dir = install_dir.path();
        cmd!(sh, "tar -zxvf {archive} -C {install_dir}").run()?;
    } else {
        let archive = &args.archive;
        let install_dir = install_dir.path();
        cmd!(sh, "7z x -y {archive} -o{install_dir}").run()?;
    }

    let scarb = find_scarb_binary(install_dir.path())?;

    cmd!(sh, "{scarb} --version").run()?;
    cmd!(sh, "{scarb} --help").run()?;

    let scarb_version = cmd!(sh, "{scarb} -V").read()?;
    ensure!(
        scarb_version.contains(expected_version),
        "wrong Scarb version, expected: {expected_version}, got: {scarb_version}",
    );

    let workdir = sh.create_temp_dir()?;
    sh.change_dir(workdir.path());
    cmd!(sh, "{scarb} new smoke_test --test-runner=none").run()?;
    sh.change_dir(workdir.path().join("smoke_test"));
    cmd!(sh, "{scarb} build").run()?;
    if args.full {
        cmd!(sh, "{scarb} lint").run()?;
        let commands = cmd!(sh, "{scarb} --json commands").read()?;
        sh.write_file("commands.json", &commands)?;
        cmd!(sh, "jq -e 'has(\"cairo-language-server\")' commands.json").run()?;
        cmd!(sh, "jq -e 'has(\"verify\")' commands.json").run()?;
        cmd!(sh, "jq -e 'has(\"prove\")' commands.json").run()?;
    }

    Ok(())
}

fn find_scarb_binary(install_dir: &Path) -> Result<PathBuf> {
    for e in WalkDir::new(install_dir) {
        let e = e?;
        if e.file_type().is_file()
            && e.file_name().to_string_lossy() == format!("scarb{}", env::consts::EXE_SUFFIX)
        {
            return Ok(e.into_path());
        }
    }

    Err(anyhow!(
        "could not find scarb{} executable in the archive",
        env::consts::EXE_SUFFIX
    ))
}
