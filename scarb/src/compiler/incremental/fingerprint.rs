use crate::compiler::incremental::source::create_local_fingerprints;
use crate::compiler::plugin::proc_macro::{
    InstanceLoader, ProcMacroPathsProvider, SharedLibraryProvider,
};
use crate::compiler::{
    CairoCompilationUnit, CompilationUnitCairoPlugin, CompilationUnitComponent,
    CompilationUnitComponentId, Profile,
};
use crate::core::{Config, ManifestCompilerConfig, Workspace};
use crate::flock::{Filesystem, LockedFile};
use crate::internal::fsx;
use crate::version::VersionInfo;
use anyhow::{Context, Result};
use cairo_lang_filesystem::cfg::CfgSet;
use cairo_lang_filesystem::db::Edition;
use camino::Utf8PathBuf;
use futures::{StreamExt, stream};
use itertools::Itertools;
use scarb_stable_hash::{StableHasher, u64_hash};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::{Arc, OnceLock, RwLock, Weak};
use tokio::task::JoinSet;
use tracing::trace_span;

/// A fingerprint is a hash that represents the state of the compilation environment for a package,
/// allowing to determine if the cache can be reused or if a recompilation is needed.
///
/// If the fingerprint is missing (the first time the unit is compiled), the cache is dirty and will not be used.
/// If the fingerprint changes, the cache is dirty and will not be used.
/// If the fingerprint is the same between compilation runs, the cache is clean and can be used.
#[derive(Debug)]
pub struct Fingerprint {
    /// Path to the Scarb binary.
    scarb_path: String,

    /// Version of Scarb and Cairo.
    scarb_version: VersionInfo,

    /// The profile used for compilation.
    profile: Profile,

    /// Name by which the component can be referred to in Cairo code.
    cairo_name: String,

    /// Cairo edition used for the component.
    edition: Edition,

    /// Component discriminator, which uniquely identifies the component within the compilation unit.
    component_discriminator: String,

    /// Compiled source paths.
    source_paths: Vec<String>,

    /// Cairo compiler configuration parameters used in the unit.
    compiler_config: ManifestCompilerConfig,

    /// Items for the Cairo's `#[cfg(...)]` attribute to be enabled for the unit.
    cfg_set: CfgSet,

    /// Experimental compiler features enabled for the component.
    experimental_features: Vec<SmolStr>,

    /// Dependencies of the component.
    deps: RwLock<Vec<DepFingerprint>>,

    /// Local files that should be checked for freshness.
    local: Vec<LocalFingerprint>,

    /// Cached fingerprint digest.
    ///
    /// Calculating digests multiple times over the span of compilation is dangerous,
    /// as the underlying inputs may change during the compilation.
    digest: OnceLock<String>,
}

/// Fingerprint of a local file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalFingerprint {
    pub path: Utf8PathBuf,
    #[serde(skip)]
    pub checksum: u64,
}

#[derive(Debug)]
pub struct PluginFingerprint {
    /// Component discriminator, which uniquely identifies the component within the compilation unit.
    component_discriminator: String,
    /// Whether the plugin is a built-in plugin or not.
    ///
    /// Builtin plugins should not have local files to check, as they are always tied to the Scarb version.
    is_builtin: bool,
    /// Whether a prebuilt plugin binary is available.
    is_prebuilt: bool,
    /// Custom fingerprint defined by the proc macro itself.
    macro_defined_fingerprint: u64,
    /// Local files that should be checked for freshness.
    local: Vec<LocalFingerprint>,
}

#[derive(Debug)]
pub struct DepFingerprint {
    /// Component discriminator, which uniquely identifies the component within the compilation unit.
    component_discriminator: String,
    /// Fingerprint created for the component.
    ///
    /// We store fingerprints as a `Weak` reference to allow cyclic dependencies.
    fingerprint: Weak<ComponentFingerprint>,
}

#[derive(Debug)]
pub enum ComponentFingerprint {
    Library(Box<Fingerprint>),
    Plugin(PluginFingerprint),
}

/// Fingerprint of all unit components.
///
/// This can be seen as a fingerprint for all inputs of a specific compilation unit,
/// although it does not differentiate by output (namely it lacks any notion of compilation targets).
pub struct UnitComponentsFingerprint(
    HashMap<CompilationUnitComponentId, Arc<ComponentFingerprint>>,
);

impl UnitComponentsFingerprint {
    #[tracing::instrument(level = "trace", skip_all)]
    pub async fn new(unit: &CairoCompilationUnit, ws: &Workspace<'_>) -> Self {
        let mut fingerprints = HashMap::new();

        enum ToFingerprint<'a> {
            Library(&'a CompilationUnitComponent),
            Plugin(&'a CompilationUnitCairoPlugin),
        }

        let mut futures = stream::iter(
            unit.components
                .iter()
                .map(ToFingerprint::Library)
                .chain(unit.cairo_plugins.iter().map(ToFingerprint::Plugin)),
        )
        .map(|component| async move {
            match component {
                ToFingerprint::Library(component) => (
                    component.id.clone(),
                    Fingerprint::try_from_component(component, unit, ws)
                        .await
                        .map(|f| ComponentFingerprint::Library(Box::new(f))),
                ),
                ToFingerprint::Plugin(plugin) => (
                    plugin.component_dependency_id.clone(),
                    PluginFingerprint::try_from_plugin(plugin, unit, ws)
                        .await
                        .map(ComponentFingerprint::Plugin),
                ),
            }
        })
        .buffer_unordered(usize::MAX);
        while let Some((id, future)) = futures.next().await {
            let fingerprint = future.expect("failed to create fingerprint for component");
            fingerprints.insert(id, Arc::new(fingerprint));
        }
        for component in unit.components.iter() {
            for dep in component
                .dependencies
                .iter()
                .map(|dep| dep.component_id())
                .sorted()
            {
                let fingerprint = fingerprints
                    .get(dep)
                    .map(Arc::downgrade)
                    .expect("component fingerprint must exist in unit fingerprints");
                let component_fingerprint = fingerprints
                    .get_mut(&component.id)
                    .expect("component fingerprint must exist in unit fingerprints");
                match &**component_fingerprint {
                    ComponentFingerprint::Library(lib) => {
                        lib.deps.write().unwrap().push(DepFingerprint {
                            component_discriminator: dep.to_crate_identifier().into(),
                            fingerprint,
                        });
                    }
                    ComponentFingerprint::Plugin(_) => {
                        panic!("plugin components should not have dependencies");
                    }
                }
            }
        }

        // Calculate digests for all fingerprints.
        let mut set = JoinSet::new();
        for fingerprint in fingerprints.values() {
            if let ComponentFingerprint::Plugin(_) = fingerprint.deref() {
                // We only care about warming up the library fingerprints.
                continue;
            };
            let fingerprint = fingerprint.clone();
            set.spawn(async move {
                fingerprint.digest();
            });
        }
        set.join_all().await;

        Self(fingerprints)
    }

    pub fn get(&self, id: &CompilationUnitComponentId) -> Option<Arc<ComponentFingerprint>> {
        self.0.get(id).cloned()
    }

    pub fn digest(&self) -> u64 {
        let mut hasher = StableHasher::new();
        hasher.write_usize(self.0.len());
        for (_, component) in self.0.iter().sorted_by_key(|(key, _)| (*key).clone()) {
            component.digest().hash(&mut hasher);
        }
        hasher.finish()
    }
}

impl ComponentFingerprint {
    pub fn digest(&self) -> String {
        match self {
            ComponentFingerprint::Library(lib) => lib.digest(),
            ComponentFingerprint::Plugin(plugin) => plugin.digest(),
        }
    }
}

impl PluginFingerprint {
    pub async fn try_from_plugin(
        component: &CompilationUnitCairoPlugin,
        _unit: &CairoCompilationUnit,
        ws: &Workspace<'_>,
    ) -> Result<Self> {
        let component_discriminator = component
            .component_dependency_id
            .to_crate_identifier()
            .into();
        let is_builtin = component.builtin;
        let is_prebuilt = component.prebuilt.is_some();
        let hash = |path: Utf8PathBuf| {
            tokio::spawn(async move {
                let span = trace_span!("plugin_local_checksum");
                let _guard = span.enter();
                let content = fsx::read(&path)
                    .with_context(|| format!("failed to read shared library at `{path}`",))?;

                anyhow::Ok(vec![LocalFingerprint {
                    path,
                    checksum: u64_hash(content),
                }])
            })
        };
        // Note that we only check built binary files. If a local plugin has changed, it would be
        // rebuilt by Cargo at this point, as we compile proc macros before Cairo compilation units.
        let local = if is_builtin {
            // Builtin plugins do not have local files to check.
            Vec::new()
        } else if is_prebuilt {
            // If the plugin is loaded from prebuilt, we do not need to check the locally built one.
            let lib_path = component.package.prebuilt_lib_path().unwrap_or_else(|| {
                unreachable!(
                    "plugin `{}` is loaded from prebuilt, but prebuilt path is not known",
                    component.package.id
                )
            });
            hash(lib_path).await??
        } else {
            let lib_path = component.shared_lib_path(ws.config())?;
            hash(lib_path).await??
        };

        let macro_defined_fingerprint = (!is_builtin)
            .then(|| {
                let instance = component.instantiate(ws.config());
                // Only v2 macros support custom fingerprints.
                anyhow::Ok::<u64>(
                    instance?
                        .try_v2()
                        .ok()
                        .map(|p| p.fingerprint())
                        .unwrap_or_default(),
                )
            })
            .transpose()?
            .unwrap_or_default();

        Ok(Self {
            component_discriminator,
            is_builtin,
            is_prebuilt,
            local,
            macro_defined_fingerprint,
        })
    }

    pub fn digest(&self) -> String {
        let mut hasher = StableHasher::new();
        self.component_discriminator.hash(&mut hasher);
        self.is_builtin.hash(&mut hasher);
        self.is_prebuilt.hash(&mut hasher);
        hasher.write_usize(self.local.len());
        for local in self.local.iter().sorted_by_key(|local| local.path.clone()) {
            local.path.hash(&mut hasher);
            local.checksum.hash(&mut hasher);
        }
        self.macro_defined_fingerprint.hash(&mut hasher);
        // HACK: turns out the `snforge-scarb-plugin` is non-deterministic.
        // To support it, we check the env variable that it uses as an input.
        // It's hardcoded here, as we need to support older versions of snforge.
        // TODO(#2444): Fix me.
        if !self.is_builtin
            && self
                .component_discriminator
                .contains("snforge_scarb_plugin")
        {
            std::env::var("SNFORGE_TEST_FILTER")
                .unwrap_or_default()
                .hash(&mut hasher);
        }
        hasher.finish_as_short_hash()
    }
}

impl Fingerprint {
    /// Create new fingerprint from component.
    ///
    /// Note: this does not fill the component dependencies!
    async fn try_from_component(
        component: &CompilationUnitComponent,
        unit: &CairoCompilationUnit,
        ws: &Workspace<'_>,
    ) -> Result<Self> {
        let scarb_path = fsx::canonicalize_utf8(ws.config().app_exe()?)?.to_string();
        let scarb_version = crate::version::get();
        let profile = ws.current_profile()?;
        let source_paths = component
            .targets
            .source_paths()
            .iter()
            .map(ToString::to_string)
            .sorted()
            .collect_vec();
        let compiler_config = unit.compiler_config.clone();
        let cfg_set = component.cfg_set.clone().unwrap_or(unit.cfg_set.clone());
        let edition = component.package.manifest.edition;
        let cairo_name = component.cairo_package_name();
        let component_discriminator = component.id.to_crate_identifier().into();
        let experimental_features = component
            .package
            .manifest
            .experimental_features
            .clone()
            .unwrap_or_default()
            .into_iter()
            .sorted()
            .collect_vec();

        let local = {
            let source_paths = component
                .targets
                .source_paths()
                .iter()
                .map(|p| p.to_path_buf())
                .collect_vec();
            let target_name = component.targets.target_name();
            let ui = ws.config().ui();
            tokio::task::spawn(
                async move { create_local_fingerprints(source_paths, target_name, ui) },
            )
            .await?
        };

        Ok(Self {
            scarb_path,
            scarb_version,
            profile,
            source_paths,
            compiler_config,
            cfg_set,
            edition,
            cairo_name,
            component_discriminator,
            experimental_features,
            local,
            deps: Default::default(),
            digest: OnceLock::new(),
        })
    }

    /// Returns a fingerprint identifier.
    ///
    /// The identifier is used to decide whether the cache should be overwritten or not, by defining
    /// the cache directory location for the component associated with this fingerprint.
    /// If a subsequent compilation run has the same identifier, it's cache's fingerprint will be
    /// checked for freshness. If it's fresh - it can be reused. If not - the cache will be
    /// overwritten.
    /// Note: this is not enough to determine if the cache can be reused or not! Please use
    /// `Fingerprint::digest` for that.
    /// Broadly speaking, the identifier is a less strict version of the digest.
    pub fn id(&self) -> String {
        // We use the set to avoid cycles when calculating digests recursively for deps.
        let mut seen = HashSet::<String>::new();
        seen.insert(self.component_discriminator.clone());
        let mut hasher = StableHasher::new();
        self.scarb_path.hash(&mut hasher);
        self.scarb_version.long().hash(&mut hasher);
        self.profile.hash(&mut hasher);
        self.cairo_name.hash(&mut hasher);
        self.edition.hash(&mut hasher);
        self.source_paths.hash(&mut hasher);
        self.compiler_config.hash(&mut hasher);
        self.experimental_features.hash(&mut hasher);
        Self::calculate_id(self, &mut seen, &mut hasher)
    }

    pub fn calculate_id(
        fingerprint: &Fingerprint,
        seen: &mut HashSet<String>,
        mut hasher: &mut StableHasher,
    ) -> String {
        fingerprint.component_discriminator.hash(hasher);
        // We hash the dependency `cfg_set` as well to accommodate compilation units for tests.
        // We emit compilation units for unit and integration tests separately.
        // In unit tests, there is a component for the main package, with `cfg(test)` enabled.
        // In integration tests, `cfg(test)` is not enabled for the main component of the
        // tested package. It's only enabled for a separate integration test component, and
        // the main package component is treated as its dependency.
        // If we did not include the `cfg_set` in the fingerprint, the cache would be
        // overwritten between unit and integration test runs.
        fingerprint.cfg_set.hash(hasher);
        let deps = fingerprint.deps.read().unwrap();
        for dep in deps
            .iter()
            .sorted_by_key(|dep| dep.component_discriminator.clone())
        {
            // Avoid dependency cycles.
            if seen.insert(dep.component_discriminator.clone()) {
                let dep_fingerprint = dep.fingerprint.upgrade()
                    .expect(
                        "dependency fingerprint should never be dropped, as long as unit fingerprint is alive"
                    );
                match dep_fingerprint.deref() {
                    ComponentFingerprint::Library(dep_fingerprint) => {
                        Self::calculate_id(dep_fingerprint.deref(), seen, hasher).hash(hasher);
                    }
                    ComponentFingerprint::Plugin(dep_fingerprint) => {
                        dep_fingerprint.component_discriminator.hash(&mut hasher);
                    }
                }
            }
        }
        hasher.finish_as_short_hash()
    }

    /// Returns a string representation of the fingerprint digest.
    ///
    /// This uniquely identifies the compilation environment for a component,
    /// allowing to determine if the cache can be reused or if a recompilation is needed.
    #[tracing::instrument(level = "trace", skip_all)]
    pub fn digest(&self) -> String {
        self.digest
            .get_or_init(|| {
                // We use the set to avoid cycles when calculating digests recursively for deps.
                let mut seen = HashSet::<String>::new();
                seen.insert(self.component_discriminator.clone());
                Self::calculate_digest(self, &mut seen)
            })
            .clone()
    }

    fn calculate_digest(fingerprint: &Fingerprint, seen: &mut HashSet<String>) -> String {
        let mut hasher = StableHasher::new();
        fingerprint.scarb_path.hash(&mut hasher);
        fingerprint.scarb_version.long().hash(&mut hasher);
        fingerprint.profile.hash(&mut hasher);
        fingerprint.cairo_name.hash(&mut hasher);
        fingerprint.edition.hash(&mut hasher);
        fingerprint.component_discriminator.hash(&mut hasher);
        fingerprint.source_paths.hash(&mut hasher);
        fingerprint.compiler_config.hash(&mut hasher);
        fingerprint.cfg_set.hash(&mut hasher);
        fingerprint.experimental_features.hash(&mut hasher);
        hasher.write_usize(fingerprint.local.len());
        for local in fingerprint
            .local
            .iter()
            .sorted_by_key(|local| local.path.clone())
        {
            local.path.hash(&mut hasher);
            local.checksum.hash(&mut hasher);
        }
        hasher.write_usize(fingerprint.deps.read().unwrap().len());
        for dep in fingerprint
            .deps
            .read()
            .unwrap()
            .iter()
            .sorted_by_key(|dep| dep.component_discriminator.clone())
        {
            // Avoid dependency cycles.
            if seen.insert(dep.component_discriminator.clone()) {
                let dep_fingerprint = dep.fingerprint.clone();
                if let Some(dep_fingerprint) = dep_fingerprint.upgrade() {
                    match dep_fingerprint.deref() {
                        ComponentFingerprint::Library(dep_fingerprint) => {
                            Self::calculate_digest(dep_fingerprint.deref(), seen).hash(&mut hasher);
                        }
                        ComponentFingerprint::Plugin(dep_fingerprint) => {
                            dep_fingerprint.digest().hash(&mut hasher);
                        }
                    }
                } else {
                    unreachable!(
                        "dependency fingerprint should never be dropped, as long as unit fingerprint is alive"
                    )
                };
            }
        }
        hasher.finish_as_short_hash()
    }
}

pub fn is_fresh(
    fingerprint_dir: &Filesystem,
    target_name: &str,
    new_digest: &str,
    config: &Config,
) -> Result<FreshnessCheck> {
    // Lock the fingerprint file for reading.
    let file_guard = fingerprint_dir
        .open_ro(target_name, target_name, config)
        .ok();

    let fingerprint_dir = fingerprint_dir.path_unchecked();
    let old_digest_path = fingerprint_dir.join(target_name);

    if !old_digest_path.exists() {
        return Ok(FreshnessCheck::not_fresh());
    }

    let old_digest = fsx::read_to_string(&old_digest_path)
        .with_context(|| format!("failed to read fingerprint from `{old_digest_path}`"))?;

    Ok(if old_digest == new_digest {
        FreshnessCheck::fresh(file_guard)
    } else {
        FreshnessCheck::not_fresh()
    })
}

pub struct FreshnessCheck {
    pub is_fresh: bool,
    pub file_guard: Option<LockedFile>,
}

impl FreshnessCheck {
    pub fn not_fresh() -> Self {
        Self {
            is_fresh: false,
            file_guard: None,
        }
    }
    pub fn fresh(file_guard: Option<LockedFile>) -> Self {
        Self {
            is_fresh: true,
            file_guard,
        }
    }
}
