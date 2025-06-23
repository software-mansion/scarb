use crate::compiler::fingerprint::{Fingerprint, is_fresh};
use crate::compiler::{CairoCompilationUnit, CompilationUnitComponent};
use crate::core::Workspace;
use anyhow::{Context, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_filesystem::db::{FilesGroup, FilesGroupEx};
use cairo_lang_filesystem::ids::BlobLongId;
use cairo_lang_lowering::cache::generate_crate_cache;
use std::env;
use std::io::Write;
use std::ops::Deref;

const SCARB_INCREMENTAL: &str = "SCARB_INCREMENTAL";

#[tracing::instrument(skip_all, level = "info")]
pub fn load_incremental_artifacts(
    db: &mut RootDatabase,
    unit: &CairoCompilationUnit,
    ws: &Workspace<'_>,
) -> Result<()> {
    if !incremental_allowed(unit) {
        return Ok(());
    }

    for component in unit.components.iter() {
        // TODO(maciektr): Enable caching for all components.
        if !component.package.id.is_core() {
            continue;
        }
        load_component_cache(db, unit, component, ws).with_context(|| {
            format!(
                "failed to load cache for `{}` component",
                component.target_name()
            )
        })?;
    }

    Ok(())
}

#[tracing::instrument(skip_all, level = "trace")]
fn load_component_cache(
    db: &mut RootDatabase,
    unit: &CairoCompilationUnit,
    component: &CompilationUnitComponent,
    ws: &Workspace<'_>,
) -> Result<()> {
    let fingerprint = Fingerprint::try_from_component(component, unit, ws).with_context(|| {
        format!(
            "failed to create fingerprint for `{}` component",
            component.target_name()
        )
    })?;

    if is_fresh(
        &fingerprint,
        &unit.fingerprint_dir(ws),
        &component.target_name(),
    )? {
        let cache_dir = unit.incremental_cache_dir(ws);
        let digest = fingerprint.digest();
        let component_id = format!("{}-{}.bin", component.target_name(), digest);
        let cache_dir = cache_dir.path_unchecked();
        let cache_file = cache_dir.join(&component_id);
        let crate_id = component.crate_id(db);
        let blob_id = db.intern_blob(BlobLongId::OnDisk(cache_file.as_std_path().to_path_buf()));
        if let Some(mut core_conf) = db.crate_config(crate_id) {
            core_conf.cache_file = Some(blob_id);
            db.set_crate_config(crate_id, Some(core_conf));
        }
    }
    Ok(())
}

#[tracing::instrument(skip_all, level = "info")]
pub fn save_incremental_artifacts(
    db: &RootDatabase,
    unit: &CairoCompilationUnit,
    ws: &Workspace<'_>,
) -> Result<()> {
    if !incremental_allowed(unit) {
        return Ok(());
    }
    for component in unit.components.iter() {
        // TODO(maciektr): Enable caching for all components.
        if !component.package.id.is_core() {
            continue;
        }
        save_component_cache(db, unit, component, ws).with_context(|| {
            format!(
                "failed to save cache for `{}` component",
                component.target_name()
            )
        })?;
    }

    Ok(())
}

#[tracing::instrument(skip_all, level = "trace")]
fn save_component_cache(
    db: &RootDatabase,
    unit: &CairoCompilationUnit,
    component: &CompilationUnitComponent,
    ws: &Workspace<'_>,
) -> Result<()> {
    let fingerprint = Fingerprint::try_from_component(component, unit, ws).with_context(|| {
        format!(
            "failed to create fingerprint for `{}` component",
            component.target_name()
        )
    })?;
    if !is_fresh(
        &fingerprint,
        &unit.fingerprint_dir(ws),
        &component.target_name(),
    )? {
        let digest = fingerprint.digest();
        let fingerprint_dir = unit.fingerprint_dir(ws);
        let cache_dir = unit.incremental_cache_dir(ws);
        let component_id = format!("{}-{}", component.target_name(), digest);
        let fingerprint_dir = fingerprint_dir.child(&component_id);
        let fingerprint_file =
            fingerprint_dir.create_rw(&component_id, "fingerprint file", ws.config())?;
        fingerprint_file
            .deref()
            .write_all(digest.as_bytes())
            .with_context(|| {
                format!(
                    "failed to write fingerprint to `{}`",
                    fingerprint_file.path()
                )
            })?;
        let cache_file =
            cache_dir.create_rw(format!("{}.bin", component_id), "cache file", ws.config())?;
        let crate_id = component.crate_id(db);
        let Some(cache_blob) = generate_crate_cache(db, crate_id).ok() else {
            return Ok(());
        };
        cache_file
            .deref()
            .write_all(&cache_blob)
            .with_context(|| format!("failed to write cache to `{}`", cache_file.path()))?;
    }
    Ok(())
}

pub fn incremental_allowed(unit: &CairoCompilationUnit) -> bool {
    let allowed_via_env = env::var(SCARB_INCREMENTAL)
        .ok()
        .map(|var| {
            let s = var.as_str();
            s == "true" || s == "1"
        })
        .unwrap_or(true);

    let allowed_via_config = unit.compiler_config.incremental;

    allowed_via_env && allowed_via_config
}
