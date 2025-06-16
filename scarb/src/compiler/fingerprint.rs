use crate::compiler::{CairoCompilationUnitWithCore, Profile};
use crate::core::{ManifestCompilerConfig, Workspace};
use crate::sources::core_version_tag;
use crate::version::VersionInfo;
use anyhow::Result;
use cairo_lang_filesystem::cfg::CfgSet;
use scarb_stable_hash::StableHasher;
use std::hash::Hash;

/// A fingerprint is a hash that represents the state of the compilation environment for a package,
/// allowing to determine if the cache can be reused or if a recompilation is needed.
///
/// If the fingerprint is missing (the first time the unit is compiled), the cache is dirty and will not be used.
/// If the fingerprint changes, the cache is dirty and will not be used.
/// If the fingerprint is the same between compilation runs, the cache is clean and can be used.
///
/// NOTE: Currently, fingerprint is limited to the corelib.
pub struct Fingerprint {
    /// Path to the Scarb binary.
    scarb_path: String,

    /// Version of Scarb and Cairo.
    scarb_version: VersionInfo,

    /// The profile used for compilation.
    profile: Profile,

    /// The version tag of the corelib.
    version_tag: String,

    /// The source path of the corelib.
    source_path: String,

    /// Cairo compiler configuration parameters used in the unit.
    compiler_config: ManifestCompilerConfig,

    /// Items for the Cairo's `#[cfg(...)]` attribute to be enabled for the unit.
    cfg_set: CfgSet,
}

impl Fingerprint {
    pub fn try_new_for_corelib(
        unit: &CairoCompilationUnitWithCore<'_>,
        ws: &Workspace<'_>,
    ) -> Result<Self> {
        let core = unit.core_package_component();
        let version_tag = core_version_tag();
        let profile = ws.current_profile()?;
        let scarb_path = ws.config().app_exe()?.to_string_lossy().to_string();
        let scarb_version = crate::version::get();
        let source_path = core.first_target().source_path.clone().to_string();
        let compiler_config = unit.compiler_config.clone();
        let cfg_set = unit.cfg_set.clone();

        Ok(Self {
            scarb_path,
            scarb_version,
            profile,
            version_tag,
            source_path,
            compiler_config,
            cfg_set,
        })
    }

    pub fn short_hash(&self) -> String {
        let mut hasher = StableHasher::new();
        self.scarb_path.hash(&mut hasher);
        self.scarb_version.long().hash(&mut hasher);
        self.profile.hash(&mut hasher);
        self.version_tag.hash(&mut hasher);
        self.source_path.hash(&mut hasher);
        self.compiler_config.hash(&mut hasher);
        self.cfg_set.hash(&mut hasher);
        hasher.finish_as_short_hash()
    }
}
