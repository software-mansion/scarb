use crate::compiler::compilation_unit::CompilationUnitAttributes;
use crate::compiler::incremental::fingerprint::{
    ComponentFingerprint, Fingerprint, FreshnessCheck, LocalFingerprint, UnitComponentsFingerprint,
    is_fresh,
};
use crate::compiler::{CairoCompilationUnit, CompilationUnitComponent};
use crate::core::Workspace;
use crate::process::is_truthy_env;
use anyhow::{Context, Result};
use cairo_lang_filesystem::db::{CrateConfiguration, FilesGroup};
use cairo_lang_filesystem::ids::{BlobLongId, CrateInput};
use cairo_lang_filesystem::set_crate_config;
use cairo_lang_lowering::cache::generate_crate_cache;
use cairo_lang_lowering::db::LoweringGroup;
use cairo_lang_utils::{CloneableDatabase, Intern};
use camino::Utf8PathBuf;
use itertools::Itertools;
use rayon::prelude::*;
use salsa::Database;
use scarb_fs_utils as fsx;
use scarb_stable_hash::u64_hash;
use std::collections::HashMap;
use std::io::{BufReader, Write};
use std::mem;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use tokio::task::spawn_blocking;
use tracing::{debug, error, trace_span};

const SCARB_INCREMENTAL: &str = "SCARB_INCREMENTAL";

/// A single warning captured during compilation, stored in the incremental cache for replay.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct CachedWarning {
    /// The diagnostic error code (e.g. `"E2066"`), if any.
    pub code: Option<String>,
    /// The formatted warning message (without trailing newline).
    pub message: String,
}

pub struct EnabledIncrementalContext {
    fingerprints: UnitComponentsFingerprint,
    cached_crates: Vec<CrateInput>,
    /// Known warnings per cached crate, keyed by `CrateInput`.
    /// Present only for crates previously compiled as a main unit.
    /// An empty `Vec` means the crate compiled with no warnings.
    cached_crate_warnings: HashMap<CrateInput, Vec<CachedWarning>>,
    /// Warnings collected during the current compilation pass (main component only).
    collected_warnings: Mutex<Vec<CachedWarning>>,
    artifacts: Mutex<Vec<LocalFingerprint>>,
}

pub enum IncrementalContext {
    Disabled,
    Enabled(Arc<EnabledIncrementalContext>),
}

impl IncrementalContext {
    pub fn enabled(&self) -> Option<Arc<EnabledIncrementalContext>> {
        let IncrementalContext::Enabled(enabled) = self else {
            return None;
        };
        Some(enabled.clone())
    }

    pub fn fingerprints(&self) -> Option<&UnitComponentsFingerprint> {
        let IncrementalContext::Enabled(enabled) = self else {
            return None;
        };
        Some(&enabled.fingerprints)
    }

    pub fn cached_crates(&self) -> &[CrateInput] {
        let IncrementalContext::Enabled(enabled) = self else {
            return &[];
        };
        &enabled.cached_crates
    }

    /// Returns the known warnings for a cached crate, or `None` if the warnings are unknown
    /// (i.e., the crate was only ever compiled as a dependency, never as a main unit).
    pub fn cached_crate_warnings_for(&self, crate_input: &CrateInput) -> Option<&[CachedWarning]> {
        let IncrementalContext::Enabled(enabled) = self else {
            return None;
        };
        enabled
            .cached_crate_warnings
            .get(crate_input)
            .map(|v| v.as_slice())
    }

    /// Records a warning emitted during the current compilation pass.
    pub fn add_warning(&self, code: Option<String>, message: String) {
        if let IncrementalContext::Enabled(enabled) = self {
            enabled
                .collected_warnings
                .lock()
                .expect("failed to acquire collected_warnings mutex")
                .push(CachedWarning { code, message });
        }
    }

    /// Returns all warnings collected during the current compilation pass.
    pub fn collected_warnings(&self) -> Vec<CachedWarning> {
        if let IncrementalContext::Enabled(enabled) = self {
            enabled
                .collected_warnings
                .lock()
                .expect("failed to acquire collected_warnings mutex")
                .clone()
        } else {
            Vec::new()
        }
    }

    pub fn register_artifact(&self, path: Utf8PathBuf) -> Result<()> {
        if let IncrementalContext::Enabled(enabled) = self {
            let content = fsx::read_to_string(&path)?;
            enabled
                .artifacts
                .lock()
                .expect("failed to acquire artifacts mutex")
                .push(LocalFingerprint {
                    path,
                    checksum: u64_hash(content),
                });
        }
        Ok(())
    }

    pub fn artifacts(&self) -> Vec<LocalFingerprint> {
        if let IncrementalContext::Enabled(enabled) = self {
            enabled
                .artifacts
                .lock()
                .expect("failed to acquire artifacts mutex")
                .to_vec()
        } else {
            Vec::new()
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ScarbComponentCache {
    /// Warnings recorded when this component was last compiled as the main unit.
    /// `None` means it was only compiled as a dependency — warnings are unknown.
    /// `Some(vec)` means it was compiled as main; `vec` holds emitted warnings (empty = clean).
    pub warnings: Option<Vec<CachedWarning>>,
    pub blob: Vec<u8>,
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
        let fingerprints = UnitComponentsFingerprint::new(unit, ws).await;
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
    let mut cached_crate_warnings: HashMap<CrateInput, Vec<CachedWarning>> = HashMap::new();
    let cached_crates = {
        let _guard = span.enter();

        loaded_components
            .into_iter()
            .filter_map(|crate_cache| match crate_cache {
                CrateCache::None => None,
                CrateCache::Loaded {
                    component,
                    blob_content,
                    warnings,
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
                    let crate_input = component.crate_input(db);
                    if let Some(known_warnings) = warnings {
                        cached_crate_warnings.insert(crate_input.clone(), known_warnings);
                    }
                    Some(crate_input)
                }
            })
            .collect_vec()
    };

    Ok(IncrementalContext::Enabled(Arc::new(
        EnabledIncrementalContext {
            fingerprints,
            cached_crates,
            cached_crate_warnings,
            collected_warnings: Default::default(),
            artifacts: Default::default(),
        },
    )))
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
        let file = cache_dir.open_ro(
            component.cache_filename(fingerprint),
            &component.cache_filename(fingerprint),
            ws.config(),
        )?;
        let mut reader = BufReader::new(file.deref());
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut bytes)?;
        let decoded: ScarbComponentCache = postcard::from_bytes(&bytes).context(format!(
            "failed to deserialize incremental cache for component {}",
            component.target_name()
        ))?;
        Ok(CrateCache::Loaded {
            component,
            warnings: decoded.warnings,
            blob_content: decoded.blob,
        })
    } else {
        Ok(CrateCache::None)
    }
}

enum CrateCache {
    None,
    Loaded {
        component: CompilationUnitComponent,
        /// `None` = was only compiled as dep (warnings unknown).
        /// `Some(vec)` = was compiled as main; `vec` holds the recorded warnings.
        warnings: Option<Vec<CachedWarning>>,
        blob_content: Vec<u8>,
    },
}

#[tracing::instrument(skip_all, level = "info")]
pub fn save_incremental_artifacts(
    unit: &CairoCompilationUnit,
    db: &dyn CloneableDatabase,
    ctx: Arc<IncrementalContext>,
    ws: &Workspace<'_>,
) -> Result<()> {
    let collected_warnings = ctx.collected_warnings();
    let Some(fingerprints) = ctx.fingerprints() else {
        return Ok(());
    };

    let main_component_id = &unit.main_component().id;
    // Determine if the main component needs a force-save: its cache blob is fresh
    // but the warnings metadata needs updating (was None, now known).
    let main_crate_input = unit.main_component().crate_input(db);
    let force_save_main = ctx.cached_crates().contains(&main_crate_input)
        && ctx.cached_crate_warnings_for(&main_crate_input).is_none();

    let components = unit
        .components
        .iter()
        .map(|component| {
            let fingerprint = fingerprints
                .get(&component.id)
                .expect("component fingerprint must exist in unit fingerprints");
            // For the main component, record the warnings we just collected.
            // For dependency components, preserve any previously known warnings so we don't
            // lose information. If a dep was never compiled as main (no entry in
            // cached_crate_warnings), keep it as None so warnings are checked the next
            // time it is compiled as the main unit.
            let is_main = &component.id == main_component_id;
            let component_warnings: Option<Vec<CachedWarning>> = if is_main {
                Some(collected_warnings.clone())
            } else {
                let crate_input = component.crate_input(db);
                if ctx.cached_crates().contains(&crate_input) {
                    // Preserve previously known warning status (may be None if dep-only).
                    ctx.cached_crate_warnings_for(&crate_input)
                        .map(|w| w.to_vec())
                } else {
                    // Compiled fresh as a dep this round; warnings unknown.
                    None
                }
            };
            let force_save = is_main && force_save_main;
            (component, fingerprint, component_warnings, force_save)
        })
        .collect_vec();

    let results: Vec<Result<()>> = components
        .par_iter()
        .map_with(
            db.dyn_clone(),
            move |group, (component, fingerprint, component_warnings, force_save)| {
                let fingerprint = match fingerprint.deref() {
                    ComponentFingerprint::Library(lib) => lib,
                    ComponentFingerprint::Plugin(_plugin) => {
                        unreachable!("we iterate through components not plugins");
                    }
                };
                save_component_cache(
                    fingerprint,
                    group.as_ref(),
                    unit,
                    component,
                    component_warnings.clone(),
                    *force_save,
                    ws,
                )
                .with_context(|| {
                    format!(
                        "failed to save cache for `{}` component",
                        component.target_name()
                    )
                })
            },
        )
        .collect();
    results.into_iter().collect::<Result<Vec<_>>>()?;

    Ok(())
}

#[tracing::instrument(skip_all, level = "trace", fields(target_name = component.target_name().to_string()))]
fn save_component_cache(
    fingerprint: &Fingerprint,
    db: &dyn CloneableDatabase,
    unit: &CairoCompilationUnit,
    component: &CompilationUnitComponent,
    warnings: Option<Vec<CachedWarning>>,
    force_save: bool,
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
    if !is_fresh || force_save {
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
        let component_cache = ScarbComponentCache {
            warnings,
            blob: cache_blob,
        };
        let cache_blob = postcard::to_allocvec(&component_cache)?;
        let cache_file = cache_dir.create_rw(
            component.cache_filename(fingerprint),
            "cache file",
            ws.config(),
        )?;
        cache_file
            .deref()
            .write_all(&cache_blob)
            .with_context(|| format!("failed to write cache to `{}`", cache_file.path()))?;
        if !is_fresh {
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
        }
    }
    Ok(())
}

trait IncrementalCachePathProvider {
    fn fingerprint_dirname(&self, fingerprint: &Fingerprint) -> String;

    fn cache_filename(&self, fingerprint: &Fingerprint) -> String;
}

impl IncrementalCachePathProvider for CompilationUnitComponent {
    fn fingerprint_dirname(&self, fingerprint: &Fingerprint) -> String {
        format!("{}-{}", self.target_name(), fingerprint.id())
    }
    fn cache_filename(&self, fingerprint: &Fingerprint) -> String {
        format!("{}.bin", self.fingerprint_dirname(fingerprint))
    }
}

pub fn incremental_allowed(unit: &CairoCompilationUnit) -> bool {
    // We allow if not explicitly disabled via the env var.
    let allowed_via_env = is_truthy_env(SCARB_INCREMENTAL, true);
    let allowed_via_config = unit.compiler_config.incremental;
    allowed_via_env && allowed_via_config
}

/// Warmup loaded crates cache in parallel.
pub fn warmup_incremental_cache(db: &dyn CloneableDatabase, cached_crates: Vec<CrateInput>) {
    let _: Vec<()> = CrateInput::into_crate_ids(db, cached_crates)
        .par_iter()
        .map_with(db.dyn_clone(), |db, crate_id| {
            let span = trace_span!("cached_multi_lowerings");
            let _guard = span.enter();
            db.cached_multi_lowerings(*crate_id);
        })
        .collect();
}
