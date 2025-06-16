use crate::compiler::CairoCompilationUnitWithCore;
use crate::compiler::Fingerprint;
use crate::core::Workspace;
use anyhow::{Context, Result};
use camino::Utf8Path;
use std::fs;
use std::io::{BufWriter, Write};
use tracing::warn;

pub fn create_corelib_fingerprint(
    unit: &CairoCompilationUnitWithCore<'_>,
    ws: &Workspace<'_>,
) -> Result<()> {
    let core = unit.core_package_component();
    let fingerprint_dir = unit.core_fingerprint_dir(ws);

    let hash_file = core.package.id.fingerprint_filename();
    let hash_file =
        fingerprint_dir.create_rw(hash_file, "corelib fingerprint file", ws.config())?;
    let mut hash_file = BufWriter::new(&*hash_file);
    let fingerprint = Fingerprint::try_new_for_corelib(unit, ws)?;
    let hash = fingerprint.short_hash();
    hash_file
        .write_all(hash.to_string().as_bytes())
        .context("failed to write corelib fingerprint")?;
    Ok(())
}

pub fn check_corelib_fingerprint_fresh(
    unit: &CairoCompilationUnitWithCore<'_>,
    ws: &Workspace<'_>,
    force_rebuild: bool,
) -> Result<bool> {
    if force_rebuild {
        return Ok(false);
    }

    let core = unit.core_package_component();
    let fingerprint_dir = unit.core_fingerprint_dir(ws);
    let fingerprint_filename = core.package.id.fingerprint_filename();
    let old_hash_path = fingerprint_dir.path_unchecked().join(&fingerprint_filename);
    if !old_hash_path.exists() {
        return Ok(false);
    }
    let old_hash_path =
        fingerprint_dir.open_ro(fingerprint_filename, "fingerprint file", ws.config())?;
    let new_fingerprint = Fingerprint::try_new_for_corelib(unit, ws)?;

    Ok(check_fingerprint_fresh(
        old_hash_path.path(),
        &new_fingerprint,
    ))
}

fn check_fingerprint_fresh(old_hash_path: &Utf8Path, new_fingerprint: &Fingerprint) -> bool {
    let old_fingerprint_hash = &fs::read_to_string(old_hash_path)
        .map_err(|_err| {
            warn!("failed to read fingerprint file: `{}`", old_hash_path);
        })
        .ok();
    let new_fingerprint_hash = new_fingerprint.short_hash();

    *old_fingerprint_hash == Some(new_fingerprint_hash)
}
