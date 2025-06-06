use crate::compiler::CairoCompilationUnit;
use crate::compiler::Fingerprint;
use crate::core::Workspace;
use crate::flock::Filesystem;
use anyhow::{Context, anyhow};
use camino::Utf8Path;
use std::fs;
use std::io::{BufWriter, Write};
use tracing::warn;

pub fn corelib_cache_dir(unit: &CairoCompilationUnit, ws: &Workspace<'_>) -> Filesystem {
    let core = unit
        .core_package_component()
        .expect("Expected core package component");
    let cache_dir = unit.incremental_dir(ws);

    let digest = unit
        .core_package_digest()
        .expect("Failed to get core package digest");
    let ident = format!("{}-{}", core.package.id.name, digest);
    cache_dir.child(ident)
}

fn corelib_fingerprint_dir(unit: &CairoCompilationUnit, ws: &Workspace<'_>) -> Option<Filesystem> {
    if let Some(core) = unit.core_package_component() {
        let digest = unit
            .core_package_digest()
            .expect("Failed to get core package digest");
        let ident = format!("{}-{}", core.package.id.name, digest);
        let fingerprint_dir = unit.fingerprint_dir(ws).child(ident);
        Some(fingerprint_dir)
    } else {
        None
    }
}

pub fn create_corelib_fingerprint(
    unit: &CairoCompilationUnit,
    ws: &Workspace<'_>,
) -> anyhow::Result<()> {
    if let Some(core) = unit.core_package_component() {
        let fingerprint_dir =
            corelib_fingerprint_dir(unit, ws).expect("Failed to get corelib fingerprint dir");

        let hash_filename = format!("{}", core.package.id.name);
        let hash_file =
            fingerprint_dir.create_rw(hash_filename, "fingerprint file", ws.config())?;
        let mut hash_file = BufWriter::new(&*hash_file);
        let fingerprint = Fingerprint::try_new_for_corelib(unit, ws)?;
        hash_file
            .write_all(fingerprint.short_hash().to_string().as_bytes())
            .context("Failed to write corelib fingerprint")?;
    }
    Ok(())
}

pub fn check_corelib_fingerprint_fresh(
    unit: &CairoCompilationUnit,
    ws: &Workspace<'_>,
    force_rebuild: bool,
) -> anyhow::Result<bool> {
    let core = unit
        .core_package_component()
        .expect("expected core package component");
    let fingerprint_dir = corelib_fingerprint_dir(unit, ws)
        .ok_or_else(|| anyhow!("Failed to get corelib fingerprint directory"))?;
    let hash_file = format!("{}", core.package.id.name);
    let old_hash_path = fingerprint_dir.path_unchecked().join(&hash_file);
    if !old_hash_path.exists() {
        return Ok(false);
    }
    let old_hash_path = fingerprint_dir.open_ro(hash_file, "fingerprint file", ws.config())?;
    let new_fingerprint = Fingerprint::try_new_for_corelib(unit, ws)?;

    let fresh = check_fingerprint_fresh(old_hash_path.path(), &new_fingerprint, force_rebuild);

    Ok(fresh)
}

fn check_fingerprint_fresh(
    old_hash_path: &Utf8Path,
    new_fingerprint: &Fingerprint,
    force_rebuild: bool,
) -> bool {
    if force_rebuild {
        return false;
    }

    let old_fingerprint_hash = &fs::read_to_string(old_hash_path)
        .map_err(|_err| {
            warn!("failed to read old fingerprint for corelib");
        })
        .ok();

    *old_fingerprint_hash == Some(new_fingerprint.short_hash())
}
