use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use xshell::{cmd, Shell};

use crate::list_binaries::list_binaries;

#[derive(Parser)]
pub struct Args {
    #[arg(short, long, env = "TARGET")]
    target: Option<String>,
    #[arg(short, long, env = "STAGING")]
    staging: PathBuf,
}

pub fn main(args: Args) -> Result<()> {
    let sh = Shell::new()?;

    let bin_dir = args.staging.join("bin");
    let doc_dir = args.staging.join("doc");

    let _ = sh.remove_path(&args.staging);
    sh.create_dir(&args.staging)?;
    sh.create_dir(&bin_dir)?;
    sh.create_dir(&doc_dir)?;

    let is_windows = args
        .target
        .as_ref()
        .map(|it| it.contains("-windows-"))
        .unwrap_or(cfg!(windows));
    let bin_ext = if is_windows { ".exe" } else { "" };

    let mut target_dir = PathBuf::from("target");
    if let Some(target) = &args.target {
        target_dir = target_dir.join(target);
    }

    for bin in list_binaries()? {
        let file_name = format!("{bin}{bin_ext}");
        sh.copy_file(
            target_dir.join("release").join(&file_name),
            bin_dir.join(file_name),
        )?;
    }

    for file in ["README.md", "SECURITY.md", "LICENSE"] {
        sh.copy_file(file, doc_dir.join(file))?;
    }

    if is_windows {
        cmd!(sh, "7z a")
            .arg(format!("{}.zip", args.staging.display()))
            .arg(&args.staging)
            .run()?;
    } else {
        cmd!(sh, "tar czvf")
            .arg(format!("{}.tar.gz", args.staging.display()))
            .arg(&args.staging)
            .run()?;
    }

    Ok(())
}
