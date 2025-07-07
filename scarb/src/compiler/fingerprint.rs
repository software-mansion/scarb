use crate::compiler::{CairoCompilationUnit, CompilationUnitComponent, Profile};
use crate::core::{ManifestCompilerConfig, Workspace};
use crate::flock::Filesystem;
use crate::internal::fsx;
use crate::version::VersionInfo;
use anyhow::{Context, Result};
use cairo_lang_filesystem::cfg::CfgSet;
use cairo_lang_filesystem::db::Edition;
use itertools::Itertools;
use scarb_stable_hash::StableHasher;
use smol_str::SmolStr;
use std::hash::Hash;

/// A fingerprint is a hash that represents the state of the compilation environment for a package,
/// allowing to determine if the cache can be reused or if a recompilation is needed.
///
/// If the fingerprint is missing (the first time the unit is compiled), the cache is dirty and will not be used.
/// If the fingerprint changes, the cache is dirty and will not be used.
/// If the fingerprint is the same between compilation runs, the cache is clean and can be used.
// TODO(maciektr): Handle information about component dependencies.
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
}

impl Fingerprint {
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
            .into_iter()
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
    pub fn digest(&self) -> String {
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
