use crate::compiler::compilation_unit::CompilationUnitAttributes;
use crate::compiler::incremental::fingerprint::{
    ComponentFingerprint, Fingerprint, FreshnessCheck, LocalFingerprint, UnitComponentsFingerprint,
    is_fresh,
};
use crate::compiler::{CairoCompilationUnit, CompilationUnitComponent, CompilationUnitComponentId};
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
use std::collections::{HashMap, HashSet};
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

/// The warning state of a cached crate.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub enum CachedWarnings {
    /// The crate was compiled as a dependency in a build — warnings were not collected.
    Unresolved,
    /// The crate was checked as a dependency via `scarb check` — no errors, but warnings were
    /// suppressed (not the main unit). Safe to skip for error diagnostics; not safe to replay
    /// as the main unit's warning output.
    Suppressed,
    /// The crate was compiled as the main unit; holds emitted warnings (empty = clean).
    Resolved(Vec<CachedWarning>),
}

/// Collects warnings emitted during a compilation or check pass.
pub struct WarningCollector(Mutex<Vec<CachedWarning>>);

impl WarningCollector {
    pub fn new() -> Self {
        Self(Mutex::new(Vec::new()))
    }

    pub fn add(&self, code: Option<String>, message: String) {
        self.0
            .lock()
            .expect("failed to acquire warning collector mutex")
            .push(CachedWarning { code, message });
    }

    pub fn collect(&self) -> Vec<CachedWarning> {
        self.0
            .lock()
            .expect("failed to acquire warning collector mutex")
            .clone()
    }
}

impl Default for WarningCollector {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BuildIncrementalContext {
    fingerprints: UnitComponentsFingerprint,
    cached_crates: Vec<CrateInput>,
    /// Components that were fresh but had no blob in their cache (written by `scarb check`).
    /// Build must regenerate and save blobs for these components.
    blob_missing_crates: HashSet<CompilationUnitComponentId>,
    /// Known warnings per cached crate, keyed by `CrateInput`.
    /// Present only for crates previously compiled as a main unit.
    /// An empty `Vec` means the crate compiled with no warnings.
    cached_crate_warnings: HashMap<CrateInput, Vec<CachedWarning>>,
    pub warning_collector: Arc<WarningCollector>,
    artifacts: Mutex<Vec<LocalFingerprint>>,
}

pub enum IncrementalContext {
    Disabled,
    Build(Arc<BuildIncrementalContext>),
    /// Check-only context: no blobs are loaded, but fresh check-verified deps are skipped.
    Check {
        /// Fingerprints loaded at the start of the check; `None` when incremental is disabled.
        fingerprints: Option<UnitComponentsFingerprint>,
        /// Cached warnings for the main component from the previous check, or `None` on a miss.
        cached_warnings: Option<Vec<CachedWarning>>,
        /// Dependency components that were verified error-free in a prior check pass.
        /// Their diagnostics can be skipped this run.
        fresh_dep_components: Vec<CompilationUnitComponent>,
        warning_collector: Arc<WarningCollector>,
    },
}

impl IncrementalContext {
    pub fn build(&self) -> Option<Arc<BuildIncrementalContext>> {
        let IncrementalContext::Build(build) = self else {
            return None;
        };
        Some(build.clone())
    }

    pub fn fingerprints(&self) -> Option<&UnitComponentsFingerprint> {
        let IncrementalContext::Build(build) = self else {
            return None;
        };
        Some(&build.fingerprints)
    }

    pub fn cached_crates(&self) -> &[CrateInput] {
        let IncrementalContext::Build(build) = self else {
            return &[];
        };
        &build.cached_crates
    }

    /// Returns the warning state for a cached crate.
    /// Returns `Unresolved` if incremental is disabled or the crate was only ever compiled as a
    /// dependency (never as a main unit).
    pub fn cached_crate_warnings_for(&self, crate_input: &CrateInput) -> CachedWarnings {
        let IncrementalContext::Build(build) = self else {
            return CachedWarnings::Unresolved;
        };
        match build.cached_crate_warnings.get(crate_input) {
            Some(v) => CachedWarnings::Resolved(v.clone()),
            None => CachedWarnings::Unresolved,
        }
    }

    /// Returns the warning collector for this context.
    pub fn warning_collector(&self) -> Option<Arc<WarningCollector>> {
        match self {
            IncrementalContext::Build(build) => Some(build.warning_collector.clone()),
            IncrementalContext::Check {
                warning_collector, ..
            } => Some(warning_collector.clone()),
            IncrementalContext::Disabled => None,
        }
    }

    /// Returns check-verified dep components whose diagnostics can be skipped this run.
    pub fn fresh_dep_components(&self) -> &[CompilationUnitComponent] {
        match self {
            IncrementalContext::Check {
                fresh_dep_components,
                ..
            } => fresh_dep_components,
            _ => &[],
        }
    }

    /// Returns the fingerprints for the check context, or `None` if disabled or not a check.
    pub fn check_fingerprints(&self) -> Option<&UnitComponentsFingerprint> {
        match self {
            IncrementalContext::Check {
                fingerprints: Some(fp),
                ..
            } => Some(fp),
            _ => None,
        }
    }

    /// Returns true if the component was fresh but had no blob in its cache
    /// (i.e. was previously written by `scarb check`).
    pub fn component_had_no_blob(&self, id: &CompilationUnitComponentId) -> bool {
        if let IncrementalContext::Build(build) = self {
            build.blob_missing_crates.contains(id)
        } else {
            false
        }
    }

    pub fn register_artifact(&self, path: Utf8PathBuf) -> Result<()> {
        if let IncrementalContext::Build(build) = self {
            let content = fsx::read_to_string(&path)?;
            build
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
        if let IncrementalContext::Build(build) = self {
            build
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
    /// Warning state recorded when this component was last compiled or checked.
    pub warnings: CachedWarnings,
    /// Cairo lowering cache blob. `None` when written by `scarb check` (no compilation output).
    pub blob: Option<Vec<u8>>,
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
    let mut blob_missing_crates: HashSet<CompilationUnitComponentId> = HashSet::new();
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
                    let crate_input = component.crate_input(db);
                    if let CachedWarnings::Resolved(known_warnings) = warnings {
                        cached_crate_warnings.insert(crate_input.clone(), known_warnings);
                    }
                    if let Some(blob) = blob_content {
                        let crate_id = component.crate_id(db);
                        if let Some(core_conf) = db.crate_config(crate_id) {
                            set_crate_config!(
                                db,
                                crate_id,
                                Some(CrateConfiguration {
                                    root: core_conf.root.clone(),
                                    settings: core_conf.settings.clone(),
                                    cache_file: Some(BlobLongId::Virtual(blob).intern(db)),
                                })
                            );
                        }
                        Some(crate_input)
                    } else {
                        // Cache was written by `scarb check` — blob not available.
                        // Track so build can generate and save the blob later.
                        blob_missing_crates.insert(component.id.clone());
                        None
                    }
                }
            })
            .collect_vec()
    };

    Ok(IncrementalContext::Build(Arc::new(
        BuildIncrementalContext {
            fingerprints,
            cached_crates,
            blob_missing_crates,
            cached_crate_warnings,
            warning_collector: Arc::new(WarningCollector::new()),
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
            blob_content: decoded.blob, // Option<Vec<u8>>
        })
    } else {
        Ok(CrateCache::None)
    }
}

enum CrateCache {
    None,
    Loaded {
        component: CompilationUnitComponent,
        warnings: CachedWarnings,
        blob_content: Option<Vec<u8>>,
    },
}

#[tracing::instrument(skip_all, level = "info")]
pub fn save_incremental_artifacts(
    unit: &CairoCompilationUnit,
    db: &dyn CloneableDatabase,
    ctx: Arc<IncrementalContext>,
    collected_warnings: Vec<CachedWarning>,
    ws: &Workspace<'_>,
) -> Result<()> {
    let Some(fingerprints) = ctx.fingerprints() else {
        return Ok(());
    };

    let main_component_id = &unit.main_component().id;

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
            // cached_crate_warnings), keep it as Unresolved so warnings are checked the next
            // time it is compiled as the main unit.
            let is_main = &component.id == main_component_id;
            let component_warnings: CachedWarnings = if is_main {
                CachedWarnings::Resolved(collected_warnings.clone())
            } else {
                let crate_input = component.crate_input(db);
                if ctx.cached_crates().contains(&crate_input) {
                    // Preserve previously known warning status (may be Unresolved if dep-only).
                    ctx.cached_crate_warnings_for(&crate_input)
                } else {
                    // Compiled fresh as a dep this round; warnings unknown.
                    CachedWarnings::Unresolved
                }
            };
            (component, fingerprint, component_warnings)
        })
        .collect_vec();

    let ctx = ctx.as_ref();
    let results: Vec<Result<()>> = components
        .par_iter()
        .map_with(
            db.dyn_clone(),
            move |group, (component, fingerprint, component_warnings)| {
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
                    ctx,
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
    warnings: CachedWarnings,
    ctx: &IncrementalContext,
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
    // Force-save when the blob is fresh but the main component's warnings just became resolved
    // for the first time (previously compiled only as a dependency, warnings were unknown).
    let is_main = component.id == unit.main_component().id;
    let force_save = is_main
        && is_fresh
        && matches!(
            ctx.cached_crate_warnings_for(&component.crate_input(db)),
            CachedWarnings::Unresolved
        );
    // Save when the component was previously written by `scarb check` without a blob.
    let needs_blob = ctx.component_had_no_blob(&component.id);
    if !is_fresh || force_save || needs_blob {
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
            blob: Some(cache_blob),
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
        // Only write the fingerprint when it didn't exist yet; for force_save and needs_blob
        // the fingerprint is already current.
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

/// Loads the check incremental cache for a unit's main component.
///
/// Returns an [`IncrementalContext::Check`] with the cached state, or [`IncrementalContext::Disabled`]
/// when incremental caching is not allowed for this unit.
#[tracing::instrument(skip_all, level = "info")]
pub fn load_check_artifacts(
    unit: &CairoCompilationUnit,
    ws: &Workspace<'_>,
) -> Result<IncrementalContext> {
    if !incremental_allowed(unit) {
        return Ok(IncrementalContext::Disabled);
    }

    let fingerprints = ws
        .config()
        .tokio_handle()
        .block_on(UnitComponentsFingerprint::new(unit, ws));

    let main_component = unit.main_component();
    let main_fingerprint = fingerprints
        .get(&main_component.id)
        .expect("component fingerprint must exist in unit fingerprints");

    let cached_warnings = load_check_component_cache(main_fingerprint, unit, main_component, ws)?;

    let mut fresh_dep_components = Vec::new();
    // Only check deps when the main component is a cache miss — on a hit we skip everything.
    if cached_warnings.is_none() {
        for component in &unit.components {
            let dep_fingerprint = fingerprints
                .get(&component.id)
                .expect("component fingerprint must exist in unit fingerprints");
            if is_dep_check_fresh(dep_fingerprint, unit, component, ws)? {
                fresh_dep_components.push(component.clone());
            }
        }
    }

    Ok(IncrementalContext::Check {
        fingerprints: Some(fingerprints),
        cached_warnings,
        fresh_dep_components,
        warning_collector: Arc::new(WarningCollector::new()),
    })
}

/// Returns `true` if a dependency component can be skipped from diagnostic checking.
///
/// Two conditions must both hold:
/// 1. The fingerprint is fresh (the dep hasn't changed since last processed).
/// 2. The cached warning state is [`CachedWarnings::Suppressed`] or [`CachedWarnings::Resolved`]
///    — meaning the dep was explicitly check-verified error-free. A build-only cache
///    ([`CachedWarnings::Unresolved`]) does not count; such deps must still be re-checked.
fn is_dep_check_fresh(
    fingerprint: Arc<ComponentFingerprint>,
    unit: &CairoCompilationUnit,
    component: &CompilationUnitComponent,
    ws: &Workspace<'_>,
) -> Result<bool> {
    let ComponentFingerprint::Library(fingerprint) = fingerprint.deref() else {
        unreachable!("we iterate through components not plugins");
    };
    let FreshnessCheck { is_fresh, .. } = is_fresh(
        &unit
            .fingerprint_dir(ws)
            .child(component.fingerprint_dirname(fingerprint)),
        &component.target_name(),
        &fingerprint.digest(),
        ws.config(),
    )?;
    if !is_fresh {
        return Ok(false);
    }
    // Fingerprint is fresh. Now verify the dep was check-verified, not just built.
    // Build-only caches carry `Unresolved`; only `Suppressed` and `Resolved` indicate
    // the dep was explicitly verified error-free by a prior `scarb check`.
    let cache_dir = unit.incremental_cache_dir(ws);
    let Ok(file) = cache_dir.open_ro(
        component.cache_filename(fingerprint),
        &component.cache_filename(fingerprint),
        ws.config(),
    ) else {
        return Ok(false);
    };
    let mut reader = BufReader::new(file.deref());
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut reader, &mut bytes)?;
    let Ok(decoded) = postcard::from_bytes::<ScarbComponentCache>(&bytes) else {
        return Ok(false);
    };
    Ok(matches!(
        decoded.warnings,
        CachedWarnings::Suppressed | CachedWarnings::Resolved(_)
    ))
}

fn load_check_component_cache(
    fingerprint: Arc<ComponentFingerprint>,
    unit: &CairoCompilationUnit,
    component: &CompilationUnitComponent,
    ws: &Workspace<'_>,
) -> Result<Option<Vec<CachedWarning>>> {
    let fingerprint = match fingerprint.deref() {
        ComponentFingerprint::Library(lib) => lib,
        ComponentFingerprint::Plugin(_) => {
            unreachable!("we iterate through components not plugins");
        }
    };
    let fingerprint_digest = fingerprint.digest();
    let FreshnessCheck {
        is_fresh,
        file_guard: _guard,
    } = is_fresh(
        &unit
            .fingerprint_dir(ws)
            .child(component.fingerprint_dirname(fingerprint)),
        &component.target_name(),
        &fingerprint_digest,
        ws.config(),
    )?;

    if !is_fresh {
        return Ok(None);
    }

    let cache_dir = unit.incremental_cache_dir(ws);
    let Ok(file) = cache_dir.open_ro(
        component.cache_filename(fingerprint),
        &component.cache_filename(fingerprint),
        ws.config(),
    ) else {
        return Ok(None);
    };
    let mut reader = BufReader::new(file.deref());
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut reader, &mut bytes)?;
    let Ok(decoded) = postcard::from_bytes::<ScarbComponentCache>(&bytes) else {
        return Ok(None);
    };

    match decoded.warnings {
        CachedWarnings::Resolved(warnings) => Ok(Some(warnings)),
        // Suppressed means the cache was written when this component was a dep — warnings were not
        // collected, so we cannot use it to skip the main-unit check.
        CachedWarnings::Suppressed | CachedWarnings::Unresolved => Ok(None),
    }
}

/// Saves a check-only incremental cache (no blob) for all components in the unit.
///
/// Only writes entries that are not already fresh — never overwrites an existing build cache.
#[tracing::instrument(skip_all, level = "info")]
pub fn save_check_artifacts(
    unit: &CairoCompilationUnit,
    ctx: &IncrementalContext,
    collected_warnings: Vec<CachedWarning>,
    ws: &Workspace<'_>,
) -> Result<()> {
    let Some(fingerprints) = ctx.check_fingerprints() else {
        return Ok(());
    };

    let main_component_id = &unit.main_component().id;

    for component in &unit.components {
        let fingerprint = fingerprints
            .get(&component.id)
            .expect("component fingerprint must exist in unit fingerprints");
        let fingerprint = match fingerprint.deref() {
            ComponentFingerprint::Library(lib) => lib,
            ComponentFingerprint::Plugin(_) => {
                unreachable!("we iterate through components not plugins");
            }
        };
        let is_main = &component.id == main_component_id;
        let warnings = if is_main {
            CachedWarnings::Resolved(collected_warnings.clone())
        } else {
            // Dep was checked alongside the main unit and had no errors. Warnings were suppressed
            // (it wasn't the main unit), but the error-free status is trustworthy and lets future
            // check runs skip re-checking this dep.
            CachedWarnings::Suppressed
        };
        save_check_component_cache(fingerprint, unit, component, warnings, ws).with_context(
            || {
                format!(
                    "failed to save check cache for `{}` component",
                    component.target_name()
                )
            },
        )?;
    }

    Ok(())
}

fn save_check_component_cache(
    fingerprint: &Fingerprint,
    unit: &CairoCompilationUnit,
    component: &CompilationUnitComponent,
    warnings: CachedWarnings,
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
    drop(file_guard);

    if is_fresh {
        // A cache already exists for this fingerprint (from a prior check or build).
        // Don't overwrite it — a build cache with a blob is more valuable.
        return Ok(());
    }

    let cache_dir = unit.incremental_cache_dir(ws);
    let component_cache = ScarbComponentCache {
        warnings,
        blob: None,
    };
    let cache_bytes = postcard::to_allocvec(&component_cache)?;
    let cache_file = cache_dir.create_rw(
        component.cache_filename(fingerprint),
        "cache file",
        ws.config(),
    )?;
    cache_file
        .deref()
        .write_all(&cache_bytes)
        .with_context(|| format!("failed to write check cache to `{}`", cache_file.path()))?;

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

    Ok(())
}
