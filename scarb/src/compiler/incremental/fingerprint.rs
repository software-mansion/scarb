use crate::compiler::incremental::source::create_local_fingerprints;
use crate::compiler::{
    CairoCompilationUnit, CompilationUnitCairoPlugin, CompilationUnitComponent,
    CompilationUnitComponentId, CompilationUnitDependency, Profile,
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
use scarb_stable_hash::StableHasher;
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
// TODO(maciektr): Handle information about filesystem changes.
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
                .sorted_by_key(|dep| dep.component_id())
            {
                let dep_component_id = match dep {
                    CompilationUnitDependency::Library(component_id) => component_id,
                    CompilationUnitDependency::Plugin(component_id) => component_id,
                };
                let fingerprint = fingerprints.get(dep_component_id);
                let fingerprint =
                    fingerprint.expect("component fingerprint must exist in unit fingerprints");
                let fingerprint = Rc::downgrade(fingerprint);
                let component_fingerprint = fingerprints
                    .get_mut(&component.id)
                    .expect("component fingerprint must exist in unit fingerprints");
                match &**component_fingerprint {
                    ComponentFingerprint::Library(lib) => {
                        lib.deps.borrow_mut().push(DepFingerprint {
                            component_discriminator: SmolStr::from(
                                dep_component_id.to_crate_identifier(),
                            ),
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
        _ws: &Workspace<'_>,
    ) -> Result<Self> {
        let component_discriminator =
            SmolStr::from(component.component_dependency_id.to_crate_identifier());
        Ok(Self {
            component_discriminator,
        })
    }

    pub fn digest(&self) -> String {
        let mut hasher = StableHasher::new();
        self.component_discriminator.hash(&mut hasher);
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
            local: create_local_fingerprints(component.targets.source_paths()),
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
        let mut hasher = StableHasher::new();
        self.scarb_path.hash(&mut hasher);
        self.scarb_version.long().hash(&mut hasher);
        self.profile.hash(&mut hasher);
        self.cairo_name.hash(&mut hasher);
        self.edition.hash(&mut hasher);
        self.component_discriminator.hash(&mut hasher);
        self.source_paths.hash(&mut hasher);
        self.compiler_config.hash(&mut hasher);
        self.cfg_set.hash(&mut hasher);
        self.experimental_features.hash(&mut hasher);
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
        Self::do_digest(self, &mut seen)
    }

    fn do_digest(fingerprint: &Fingerprint, seen: &mut HashSet<SmolStr>) -> String {
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
                            Self::do_digest(dep_fingerprint.deref(), seen).hash(&mut hasher);
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
    fingerprint_dirname: &str,
    target_name: &str,
    new_digest: &str,
) -> Result<bool> {
    let fingerprint_dir = fingerprint_dir.child(fingerprint_dirname);
    let fingerprint_dir = fingerprint_dir.path_unchecked();
    let old_digest_path = fingerprint_dir.join(target_name);

    if !old_digest_path.exists() {
        return Ok(false);
    }

    let old_digest = fsx::read_to_string(&old_digest_path)
        .with_context(|| format!("failed to read fingerprint from `{old_digest_path}`"))?;

    Ok(old_digest == new_digest)
}
