#![allow(dead_code)]

use crate::compiler::{CairoCompilationUnit, CompilationUnitComponent, Profile};
use crate::core::{ManifestCompilerConfig, Workspace};
use crate::internal::fsx;
use crate::version::VersionInfo;
use anyhow::Result;
use cairo_lang_filesystem::cfg::CfgSet;
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
pub struct Fingerprint {
    /// Path to the Scarb binary.
    scarb_path: String,

    /// Version of Scarb and Cairo.
    scarb_version: VersionInfo,

    /// The profile used for compilation.
    profile: Profile,

    /// Name by which the component can be referred to in Cairo code.
    cairo_name: SmolStr,

    /// Component discriminator, which uniquely identifies the component within the compilation unit.
    component_discriminator: SmolStr,

    /// Compiled source paths.
    source_paths: Vec<String>,

    /// Cairo compiler configuration parameters used in the unit.
    compiler_config: ManifestCompilerConfig,

    /// Items for the Cairo's `#[cfg(...)]` attribute to be enabled for the unit.
    cfg_set: CfgSet,
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
        let source_paths = component.targets.source_paths();
        let compiler_config = unit.compiler_config.clone();
        let cfg_set = component.cfg_set.clone().unwrap_or(unit.cfg_set.clone());
        let cairo_name = component.cairo_package_name();
        let component_discriminator = SmolStr::from(component.id.to_crate_identifier());
        Ok(Self {
            scarb_path,
            scarb_version,
            profile,
            source_paths,
            compiler_config,
            cfg_set,
            cairo_name,
            component_discriminator,
        })
    }

    pub fn digest(&self) -> String {
        let mut hasher = StableHasher::new();
        self.scarb_path.hash(&mut hasher);
        self.scarb_version.long().hash(&mut hasher);
        self.profile.hash(&mut hasher);
        self.component_discriminator.hash(&mut hasher);
        self.source_paths.hash(&mut hasher);
        self.compiler_config.hash(&mut hasher);
        self.cfg_set.hash(&mut hasher);
        hasher.finish_as_short_hash()
    }
}
