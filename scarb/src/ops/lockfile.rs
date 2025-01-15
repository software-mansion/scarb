use crate::core::lockfile::Lockfile;
use crate::core::Workspace;
use anyhow::{Context, Result};
use fs4::FileExt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::str::FromStr;

#[tracing::instrument(skip_all, level = "debug")]
pub fn read_lockfile(ws: &Workspace<'_>) -> Result<Lockfile> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(ws.lockfile_path())
        .context("failed to open lockfile")?;

    FileExt::lock_shared(&file).context("failed to acquire shared lockfile access")?;

    let mut content = String::new();
    file.read_to_string(&mut content)?;

    Lockfile::from_str(&content)
}

#[tracing::instrument(skip_all, level = "debug")]
pub fn write_lockfile(lockfile: Lockfile, ws: &Workspace<'_>) -> Result<()> {
    let mut file = File::create(ws.lockfile_path()).context("failed to create lockfile")?;

    file.lock_exclusive()
        .context("failed to acquire exclusive lockfile access")?;

    file.write_all(lockfile.render()?.as_bytes())
        .context("failed to write lockfile content")?;

    Ok(())
}
