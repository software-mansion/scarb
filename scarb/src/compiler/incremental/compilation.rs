use crate::compiler::incremental::fingerprint::{
    ComponentFingerprint, Fingerprint, FreshnessCheck, UnitFingerprint, is_fresh,
};
use crate::compiler::{CairoCompilationUnit, CompilationUnitComponent};
use crate::core::Workspace;
use crate::internal::fsx;
use anyhow::{Context, Result};
use cairo_lang_filesystem::db::{CrateConfiguration, FilesGroup};
use cairo_lang_filesystem::ids::{BlobLongId, CrateInput};
use cairo_lang_filesystem::set_crate_config;
use cairo_lang_lowering::cache::generate_crate_cache;
use cairo_lang_lowering::db::LoweringGroup;
use cairo_lang_utils::Intern;
use itertools::Itertools;
use salsa::{Database, par_map};
use std::io::Write;
use std::ops::Deref;
use std::sync::Arc;
use std::{env, mem};
use tokio::task::spawn_blocking;
use tracing::{debug, error, trace_span};

const SCARB_INCREMENTAL: &str = "SCARB_INCREMENTAL";

pub enum IncrementalContext {
    Disabled,
    Enabled {
        fingerprints: UnitFingerprint,
        cached_crates: Vec<CrateInput>,
    },
}

impl IncrementalContext {
    pub fn cached_crates(&self) -> &[CrateInput] {
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
    db: &mut dyn Database,
    ws: &Workspace<'_>,
) -> Result<IncrementalContext> {
    if !incremental_allowed(unit) {
        return Ok(IncrementalContext::Disabled);
    }

    let (fingerprints, loaded_components) = ws.config().tokio_handle().block_on(async {
        let fingerprints = UnitFingerprint::new(unit, ws).await;
        let handles = unit
            .components
            .iter()
            .map(|component| {
                let fingerprint = fingerprints
                    .get(&component.id)
                    .expect("component fingerprint must exist in unit fingerprints");
                let handle = {
                    // HACK: We know that we will not use &Workspace outside the scope of this function,
                    //   but `tokio::spawn_blocking` lifetime bounds force us to think so.
                    let ws: &'static Workspace<'_> = unsafe { mem::transmute(ws) };
                    let unit = unit.clone();
                    let component = component.clone();
                    spawn_blocking(move || load_component_cache(fingerprint, unit, component, ws))
                };
                (handle, component)
            })
            .collect_vec();

        let mut caches = Vec::new();
        for (handle, component) in handles {
            let loaded = handle.await?.with_context(|| {
                format!(
                    "failed to load cache for `{}` component",
                    component.target_name()
                )
            })?;
            caches.push(loaded);
        }

        anyhow::Ok((fingerprints, caches))
    })?;

    let span = trace_span!("set_crate_configs");
    let cached_crates = {
        let _guard = span.enter();

        loaded_components
            .into_iter()
            .filter_map(|crate_cache| match crate_cache {
                CrateCache::None => None,
                CrateCache::Loaded {
                    component,
                    blob_content,
                } => {
                    let crate_id = component.crate_id(db);
                    if let Some(core_conf) = db.crate_config(crate_id) {
                        set_crate_config!(
                            db,
                            crate_id,
                            Some(CrateConfiguration {
                                root: core_conf.root.clone(),
                                settings: core_conf.settings.clone(),
                                cache_file: Some(BlobLongId::Virtual(blob_content).intern(db)),
                            })
                        );
                    }
                    Some(component.crate_input(db))
                }
            })
            .collect_vec()
    };

    // Warmup loaded crates cache in parallel.
    let _: Vec<()> = par_map(
        db,
        CrateInput::into_crate_ids(db, cached_crates.clone()),
        |db, crate_id| {
            let span = trace_span!("cached_multi_lowerings");
            let _guard = span.enter();
            db.cached_multi_lowerings(crate_id);
        },
    );

    Ok(IncrementalContext::Enabled {
        fingerprints,
        cached_crates,
    })
}

/// Loads the cache for a specific component if it is fresh.
///
/// Returns `Ok(true)` if the cache was loaded successfully, or `Ok(false)` if the component
/// is not fresh and no cache was loaded.
#[tracing::instrument(skip_all, level = "trace", fields(target_name = component.target_name().to_string()))]
fn load_component_cache(
    fingerprint: Arc<ComponentFingerprint>,
    unit: CairoCompilationUnit,
    component: CompilationUnitComponent,
    ws: &Workspace<'_>,
) -> Result<CrateCache> {
    let fingerprint = match fingerprint.deref() {
        ComponentFingerprint::Library(lib) => lib,
        ComponentFingerprint::Plugin(_plugin) => {
            unreachable!("we iterate through components not plugins");
        }
    };
    let fingerprint_digest = fingerprint.digest();
    let FreshnessCheck {
        is_fresh,
        // We keep the fingerprint lock, so no cache writes can occur between
        // freshness check and cache load.
        file_guard: _guard,
    } = is_fresh(
        &unit
            .fingerprint_dir(ws)
            .child(component.fingerprint_dirname(fingerprint)),
        &component.target_name(),
        &fingerprint_digest,
        ws.config(),
    )?;
    if is_fresh {
        debug!(
            "component `{}` is fresh, loading cache artifacts",
            component.target_name()
        );
        let cache_dir = unit.incremental_cache_dir(ws);
        // Lock the cache file for reading.
        let _guard = cache_dir.open_ro(
            component.cache_filename(fingerprint),
            &component.cache_filename(fingerprint),
            ws.config(),
        );
        let cache_dir = cache_dir.path_unchecked();
        let cache_file = cache_dir.join(component.cache_filename(fingerprint));
        let blob_content = fsx::read(cache_file)?;
        Ok(CrateCache::Loaded {
            component,
            blob_content,
        })
    } else {
        Ok(CrateCache::None)
    }
}

enum CrateCache {
    None,
    Loaded {
        component: CompilationUnitComponent,
        blob_content: Vec<u8>,
    },
}

#[tracing::instrument(skip_all, level = "info")]
pub fn save_incremental_artifacts(
    unit: &CairoCompilationUnit,
    db: &dyn Database,
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

    let results: Vec<Result<()>> =
        par_map(db, components, move |group, (component, fingerprint)| {
            let fingerprint = match fingerprint.deref() {
                ComponentFingerprint::Library(lib) => lib,
                ComponentFingerprint::Plugin(_plugin) => {
                    unreachable!("we iterate through components not plugins");
                }
            };
            save_component_cache(fingerprint, group, unit, component, ws).with_context(|| {
                format!(
                    "failed to save cache for `{}` component",
                    component.target_name()
                )
            })
        });
    results.into_iter().collect::<Result<Vec<_>>>()?;

    Ok(())
}

#[tracing::instrument(skip_all, level = "trace", fields(target_name = component.target_name().to_string()))]
fn save_component_cache(
    fingerprint: &Fingerprint,
    db: &dyn Database,
    unit: &CairoCompilationUnit,
    component: &CompilationUnitComponent,
    ws: &Workspace<'_>,
) -> Result<()> {
    let fingerprint_digest = fingerprint.digest();
    let FreshnessCheck {
        is_fresh,
        file_guard,
    } = is_fresh(
        &unit
            .fingerprint_dir(ws)
            .child(component.fingerprint_dirname(fingerprint)),
        &component.target_name(),
        &fingerprint_digest,
        ws.config(),
    )?;
    // We drop the fingerprint lock, so it can be locked for writing.
    drop(file_guard);
    if !is_fresh {
        debug!(
            "component `{}` is not fresh, saving new cache artifacts",
            component.target_name()
        );
        let cache_dir = unit.incremental_cache_dir(ws);
        let crate_id = component.crate_id(db);
        let cache_blob = match generate_crate_cache(db, crate_id) {
            Ok(blob) => blob,
            Err(_e) => {
                error!(
                    "failed to generate cache for `{}` crate",
                    component.target_name()
                );
                return Ok(());
            }
        };
        let fingerprint_dir = unit.fingerprint_dir(ws);
        let fingerprint_dir = fingerprint_dir.child(component.fingerprint_dirname(fingerprint));
        let fingerprint_file = fingerprint_dir.create_rw(
            component.target_name().as_str(),
            "fingerprint file",
            ws.config(),
        )?;
        let cache_file = cache_dir.create_rw(
            component.cache_filename(fingerprint),
            "cache file",
            ws.config(),
        )?;
        cache_file
            .deref()
            .write_all(&cache_blob)
            .with_context(|| format!("failed to write cache to `{}`", cache_file.path()))?;
        fingerprint_file
            .deref()
            .write_all(fingerprint_digest.as_bytes())
            .with_context(|| {
                format!(
                    "failed to write fingerprint to `{}`",
                    fingerprint_file.path()
                )
            })?;
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
