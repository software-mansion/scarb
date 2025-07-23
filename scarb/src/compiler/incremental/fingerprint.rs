use crate::compiler::incremental::source::create_local_fingerprints;
use crate::compiler::plugin::proc_macro::{ProcMacroPathsProvider, SharedLibraryProvider};
use crate::compiler::{
    CairoCompilationUnit, CompilationUnitCairoPlugin, CompilationUnitComponent,
    CompilationUnitComponentId, Profile,
};
use crate::core::{ManifestCompilerConfig, Workspace};
use crate::flock::Filesystem;
use crate::internal::fsx;
use crate::version::VersionInfo;
use anyhow::{Context, Result};
use cairo_lang_filesystem::cfg::CfgSet;
use cairo_lang_filesystem::db::Edition;
use camino::Utf8PathBuf;
use itertools::Itertools;
use scarb_stable_hash::{StableHasher, u64_hash};
use smol_str::SmolStr;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::{Rc, Weak};

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
    cairo_name: SmolStr,

    /// Cairo edition used for the component.
    edition: Edition,

    /// Component discriminator, which uniquely identifies the component within the compilation unit.
    component_discriminator: SmolStr,

    /// Compiled source paths.
    source_paths: Vec<String>,

    /// Cairo compiler configuration parameters used in the unit.
    compiler_config: ManifestCompilerConfig,

    /// Items for the Cairo's `#[cfg(...)]` attribute to be enabled for the unit.
    cfg_set: CfgSet,

    /// Experimental compiler features enabled for the component.
    experimental_features: Vec<SmolStr>,

    /// Dependencies of the component.
    deps: RefCell<Vec<DepFingerprint>>,

    /// Local files that should be checked for freshness.
    local: Vec<LocalFingerprint>,
}

#[derive(Debug)]
pub struct LocalFingerprint {
    pub path: Utf8PathBuf,
    pub checksum: u64,
}

#[derive(Debug)]
pub struct PluginFingerprint {
    /// Component discriminator, which uniquely identifies the component within the compilation unit.
    component_discriminator: SmolStr,
    /// Whether the plugin is a built-in plugin or not.
    ///
    /// Builtin plugins should not have local files to check, as they are always tied to the Scarb version.
    is_builtin: bool,
    /// Whether a prebuilt plugin binary is available.
    is_prebuilt: bool,
    /// Local files that should be checked for freshness.
    local: Vec<LocalFingerprint>,
}

#[derive(Debug)]
pub struct DepFingerprint {
    /// Component discriminator, which uniquely identifies the component within the compilation unit.
    component_discriminator: SmolStr,
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

pub struct UnitFingerprint(HashMap<CompilationUnitComponentId, Rc<ComponentFingerprint>>);

impl UnitFingerprint {
    #[tracing::instrument(level = "trace", skip_all)]
    pub fn new(unit: &CairoCompilationUnit, ws: &Workspace<'_>) -> Self {
        let mut fingerprints = HashMap::new();
        for component in unit.components.iter() {
            let fingerprint = Fingerprint::try_from_component(component, unit, ws)
                .expect("failed to create fingerprint for component");
            fingerprints.insert(
                component.id.clone(),
                Rc::new(ComponentFingerprint::Library(Box::new(fingerprint))),
            );
        }
        for plugin in unit.cairo_plugins.iter() {
            let fingerprint = PluginFingerprint::try_from_plugin(plugin, unit, ws)
                .expect("failed to create fingerprint for plugin");
            fingerprints.insert(
                plugin.component_dependency_id.clone(),
                Rc::new(ComponentFingerprint::Plugin(fingerprint)),
            );
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
                    .map(Rc::downgrade)
                    .expect("component fingerprint must exist in unit fingerprints");
                let component_fingerprint = fingerprints
                    .get_mut(&component.id)
                    .expect("component fingerprint must exist in unit fingerprints");
                match &**component_fingerprint {
                    ComponentFingerprint::Library(lib) => {
                        lib.deps.borrow_mut().push(DepFingerprint {
                            component_discriminator: SmolStr::from(dep.to_crate_identifier()),
                            fingerprint,
                        });
                    }
                    ComponentFingerprint::Plugin(_) => {
                        panic!("plugin components should not have dependencies");
                    }
                }
            }
        }
        Self(fingerprints)
    }

    pub fn get(&self, id: &CompilationUnitComponentId) -> Option<Rc<ComponentFingerprint>> {
        self.0.get(id).cloned()
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
    pub fn try_from_plugin(
        component: &CompilationUnitCairoPlugin,
        _unit: &CairoCompilationUnit,
        ws: &Workspace<'_>,
    ) -> Result<Self> {
        let component_discriminator =
            SmolStr::from(component.component_dependency_id.to_crate_identifier());
        let is_builtin = component.builtin;
        let is_prebuilt = component.prebuilt.is_some();
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
            let content = fsx::read(&lib_path)
                .with_context(|| format!("failed to read shared library at `{lib_path}`",))?;
            vec![LocalFingerprint {
                path: lib_path,
                checksum: u64_hash(content),
            }]
        } else {
            let lib_path = component.shared_lib_path(ws.config())?;
            let content = fsx::read(&lib_path)
                .with_context(|| format!("failed to read shared library at `{lib_path}`",))?;
            vec![LocalFingerprint {
                path: lib_path,
                checksum: u64_hash(content),
            }]
        };
        Ok(Self {
            component_discriminator,
            is_builtin,
            is_prebuilt,
            local,
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
        // HACK: turns out the `snforge-scarb-plugin` is non-deterministic.
        // To support it, we check the env variable that it uses as an input.
        // It's hardcoded here, as we need to support older versions of snforge.
        // TODO(maciektr): Remove this hack.
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
    pub fn try_from_component(
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
        let component_discriminator = SmolStr::from(component.id.to_crate_identifier());
        let experimental_features = component
            .package
            .manifest
            .experimental_features
            .clone()
            .unwrap_or_default()
            .into_iter()
            .sorted()
            .collect_vec();
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
            local: create_local_fingerprints(
                component.targets.source_paths(),
                component.targets.target_name(),
                &ws.config().ui(),
            ),
            deps: Default::default(),
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
        let mut seen = HashSet::<SmolStr>::new();
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
        seen: &mut HashSet<SmolStr>,
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
        for dep in fingerprint
            .deps
            .borrow()
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
        // We use the set to avoid cycles when calculating digests recursively for deps.
        let mut seen = HashSet::<SmolStr>::new();
        seen.insert(self.component_discriminator.clone());
        Self::calculate_digest(self, &mut seen)
    }

    fn calculate_digest(fingerprint: &Fingerprint, seen: &mut HashSet<SmolStr>) -> String {
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
        hasher.write_usize(fingerprint.deps.borrow().len());
        for dep in fingerprint
            .deps
            .borrow()
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

pub fn is_fresh(fingerprint_dir: &Filesystem, target_name: &str, new_digest: &str) -> Result<bool> {
    let fingerprint_dir = fingerprint_dir.path_unchecked();
    let old_digest_path = fingerprint_dir.join(target_name);

    if !old_digest_path.exists() {
        return Ok(false);
    }

    let old_digest = fsx::read_to_string(&old_digest_path)
        .with_context(|| format!("failed to read fingerprint from `{old_digest_path}`"))?;

    Ok(old_digest == new_digest)
}
