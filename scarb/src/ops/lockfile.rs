use crate::core::lockfile::Lockfile;
use crate::core::Workspace;
use anyhow::{Context, Result};
use fs4::FileExt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

#[tracing::instrument(skip_all, level = "debug")]
pub fn read_lockfile(ws: &Workspace<'_>) -> Result<Lockfile> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(ws.lockfile_path())
        .context("failed to open lockfile")?;

    file.lock_shared()
        .context("failed to acquire shared lockfile access")?;

    let mut content = String::new();
    file.read_to_string(&mut content)?;

    content.try_into()
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
