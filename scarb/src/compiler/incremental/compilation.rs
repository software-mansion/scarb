use crate::compiler::incremental::fingerprint::{
    ComponentFingerprint, Fingerprint, UnitFingerprint, is_fresh,
};
use crate::compiler::{CairoCompilationUnit, CompilationUnitComponent};
use crate::core::Workspace;
use anyhow::{Context, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_filesystem::db::{FilesGroup, FilesGroupEx};
use cairo_lang_filesystem::ids::{BlobLongId, CrateId};
use cairo_lang_lowering::cache::generate_crate_cache;
use itertools::Itertools;
use std::env;
use std::io::Write;
use std::ops::Deref;
use tracing::debug;

const SCARB_INCREMENTAL: &str = "SCARB_INCREMENTAL";

pub enum IncrementalContext {
    Disabled,
    Enabled {
        fingerprints: UnitFingerprint,
        cached_crates: Vec<CrateId>,
    },
}

impl IncrementalContext {
    pub fn cached_crates(&self) -> &[CrateId] {
        match self {
            IncrementalContext::Disabled => &[],
            IncrementalContext::Enabled {
                fingerprints: _,
                cached_crates,
            } => cached_crates,
        }
    }
}

#[tracing::instrument(skip_all, level = "info")]
pub fn load_incremental_artifacts(
    unit: &CairoCompilationUnit,
    db: &mut RootDatabase,
    ws: &Workspace<'_>,
) -> Result<IncrementalContext> {
    if !incremental_allowed(unit) {
        return Ok(IncrementalContext::Disabled);
    }

    let fingerprints = ws
        .config()
        .tokio_handle()
        .block_on(UnitFingerprint::new(unit, ws));

    let mut cached_crates = Vec::new();

    for component in unit.components.iter() {
        let fingerprint = fingerprints
            .get(&component.id)
            .expect("component fingerprint must exist in unit fingerprints");
        let fingerprint = match fingerprint.deref() {
            ComponentFingerprint::Library(lib) => lib,
            ComponentFingerprint::Plugin(_plugin) => {
                unreachable!("we iterate through components not plugins");
            }
        };
        let loaded =
            load_component_cache(fingerprint, db, unit, component, ws).with_context(|| {
                format!(
                    "failed to load cache for `{}` component",
                    component.target_name()
                )
            })?;
        if loaded {
            cached_crates.push(component.crate_id(db));
        }
    }

    Ok(IncrementalContext::Enabled {
        fingerprints,
        cached_crates,
    })
}

/// Loads the cache for a specific component if it is fresh.
///
/// Returns `Ok(true)` if the cache was loaded successfully, or `Ok(false)` if the component
/// is not fresh and no cache was loaded.
#[tracing::instrument(skip_all, level = "trace")]
fn load_component_cache(
    fingerprint: &Fingerprint,
    db: &mut RootDatabase,
    unit: &CairoCompilationUnit,
    component: &CompilationUnitComponent,
    ws: &Workspace<'_>,
) -> Result<bool> {
    let fingerprint_digest = fingerprint.digest();
    if is_fresh(
        &unit
            .fingerprint_dir(ws)
            .child(component.fingerprint_dirname(fingerprint)),
        &component.target_name(),
        &fingerprint_digest,
    )? {
        debug!(
            "component `{}` is fresh, loading cache artifacts",
            component.target_name()
        );
        let cache_dir = unit.incremental_cache_dir(ws);
        let cache_dir = cache_dir.path_unchecked();
        let cache_file = cache_dir.join(component.cache_filename(fingerprint));
        let crate_id = component.crate_id(db);
        let blob_id = db.intern_blob(BlobLongId::OnDisk(cache_file.as_std_path().to_path_buf()));
        if let Some(mut core_conf) = db.crate_config(crate_id) {
            core_conf.cache_file = Some(blob_id);
            db.set_crate_config(crate_id, Some(core_conf));
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tracing::instrument(skip_all, level = "info")]
pub fn save_incremental_artifacts(
    unit: &CairoCompilationUnit,
    db: &RootDatabase,
    ctx: IncrementalContext,
    ws: &Workspace<'_>,
) -> Result<()> {
    let IncrementalContext::Enabled {
        fingerprints,
        cached_crates: _,
    } = ctx
    else {
        return Ok(());
    };

    let components = unit
        .components
        .iter()
        .map(|component| {
            let fingerprint = fingerprints
                .get(&component.id)
                .expect("component fingerprint must exist in unit fingerprints");
            (component, fingerprint)
        })
        .collect_vec();

    let snapshot = salsa::ParallelDatabase::snapshot(db);
    rayon::scope(move |s| {
        for (component, fingerprint) in components.into_iter() {
            let snapshot = salsa::ParallelDatabase::snapshot(&*snapshot);
            s.spawn(move |_| {
                let fingerprint = match fingerprint.deref() {
                    ComponentFingerprint::Library(lib) => lib,
                    ComponentFingerprint::Plugin(_plugin) => {
                        unreachable!("we iterate through components not plugins");
                    }
                };
                save_component_cache(fingerprint, snapshot, unit, component, ws)
                    .with_context(|| {
                        format!(
                            "failed to save cache for `{}` component",
                            component.target_name()
                        )
                    })
                    .unwrap();
            });
        }
    });

    Ok(())
}

#[tracing::instrument(skip_all, level = "trace")]
fn save_component_cache(
    fingerprint: &Fingerprint,
    db: salsa::Snapshot<RootDatabase>,
    unit: &CairoCompilationUnit,
    component: &CompilationUnitComponent,
    ws: &Workspace<'_>,
) -> Result<()> {
    let fingerprint_digest = fingerprint.digest();
    if !is_fresh(
        &unit
            .fingerprint_dir(ws)
            .child(component.fingerprint_dirname(fingerprint)),
        &component.target_name(),
        &fingerprint_digest,
    )? {
        debug!(
            "component `{}` is not fresh, saving new cache artifacts",
            component.target_name()
        );
        let fingerprint_dir = unit.fingerprint_dir(ws);
        let fingerprint_dir = fingerprint_dir.child(component.fingerprint_dirname(fingerprint));
        let fingerprint_file = fingerprint_dir.create_rw(
            component.target_name().as_str(),
            "fingerprint file",
            ws.config(),
        )?;
        fingerprint_file
            .deref()
            .write_all(fingerprint_digest.as_bytes())
            .with_context(|| {
                format!(
                    "failed to write fingerprint to `{}`",
                    fingerprint_file.path()
                )
            })?;
        let cache_dir = unit.incremental_cache_dir(ws);
        let cache_file = cache_dir.create_rw(
            component.cache_filename(fingerprint),
            "cache file",
            ws.config(),
        )?;
        let crate_id = component.crate_id(&*db);
        let Some(cache_blob) = generate_crate_cache(&*db, crate_id).ok() else {
            return Ok(());
        };
        cache_file
            .deref()
            .write_all(&cache_blob)
            .with_context(|| format!("failed to write cache to `{}`", cache_file.path()))?;
    }
    Ok(())
}

trait IncrementalArtifactsProvider {
    fn fingerprint_dirname(&self, fingerprint: &Fingerprint) -> String;

    fn cache_filename(&self, fingerprint: &Fingerprint) -> String;
}

impl IncrementalArtifactsProvider for CompilationUnitComponent {
    fn fingerprint_dirname(&self, fingerprint: &Fingerprint) -> String {
        format!("{}-{}", self.target_name(), fingerprint.id())
    }
    fn cache_filename(&self, fingerprint: &Fingerprint) -> String {
        format!("{}.bin", self.fingerprint_dirname(fingerprint))
    }
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
