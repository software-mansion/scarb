use crate::compiler::plugin::proc_macro;
use crate::compiler::plugin::proc_macro::SharedLibraryProvider;
use crate::compiler::plugin::proc_macro::expansion::{Expansion, ExpansionKind};
use crate::compiler::plugin::proc_macro::ffi::SharedPluginLibrary;
use crate::core::{Package, PackageId};
use anyhow::{Context, Result, anyhow};
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::fmt::Debug;
use tracing::trace;

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(rename_all = "lowercase")]
pub enum ProcMacroApiVersion {
    #[default]
    V1,
    V2,
}

/// Representation of a single, loaded procedural macro package.
///
/// This struct holds Scarb metadata of a proc macro package (package id, available expansions)
/// and a loaded plugin instance.
pub struct ProcMacroInstance {
    package_id: PackageId,
    plugin: VersionedPlugin,
    expansions: Vec<Expansion>,
}

impl Debug for ProcMacroInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcMacroInstance")
            .field("package_id", &self.package_id)
            .finish()
    }
}

impl ProcMacroInstance {
    /// Load shared library
    pub fn try_new(package: &Package, lib_path: Utf8PathBuf) -> Result<Self> {
        trace!("loading compiled macro for `{}` package", package.id);
        let plugin = unsafe { VersionedPlugin::try_new(package, &lib_path)? };
        Ok(Self {
            expansions: unsafe { plugin.load_expansions(package.id)? },
            package_id: package.id,
            plugin,
        })
    }

    pub fn try_load_prebuilt(package: Package) -> Result<Self> {
        trace!("loading prebuilt macro for `{}` package", package.id);
        let prebuilt_path = package
            .prebuilt_lib_path()
            .context("could not resolve prebuilt library path")?;
        let plugin = unsafe { VersionedPlugin::try_new(&package, &prebuilt_path)? };
        Ok(Self {
            expansions: unsafe { plugin.load_expansions(package.id)? },
            package_id: package.id,
            plugin,
        })
    }

    pub fn package_id(&self) -> PackageId {
        self.package_id
    }

    pub fn get_expansions(&self) -> &[Expansion] {
        &self.expansions
    }

    fn plugin(&self) -> &VersionedPlugin {
        &self.plugin
    }

    pub fn declared_attributes_and_executables(&self) -> Vec<String> {
        self.get_expansions()
            .iter()
            .filter(|e| e.kind == ExpansionKind::Attr || e.kind == ExpansionKind::Executable)
            .map(|e| e.name.clone())
            .map(Into::into)
            .collect()
    }

    pub fn declared_attributes(&self) -> Vec<String> {
        self.get_expansions()
            .iter()
            .filter(|e| e.kind == ExpansionKind::Attr)
            .map(|e| e.name.clone())
            .map(Into::into)
            .collect()
    }

    pub fn declared_derives(&self) -> Vec<String> {
        self.get_expansions()
            .iter()
            .filter(|e| e.kind == ExpansionKind::Derive)
            .map(|e| e.name.clone())
            .map(Into::into)
            .collect()
    }

    pub fn executable_attributes(&self) -> Vec<String> {
        self.get_expansions()
            .iter()
            .filter(|e| e.kind == ExpansionKind::Executable)
            .map(|e| e.name.clone())
            .map(Into::into)
            .collect()
    }

    pub fn inline_macros(&self) -> Vec<String> {
        self.get_expansions()
            .iter()
            .filter(|e| e.kind == ExpansionKind::Inline)
            .map(|e| e.name.clone())
            .map(Into::into)
            .collect()
    }

    pub fn doc(&self, item_name: SmolStr) -> Option<String> {
        self.plugin().doc(item_name)
    }

    pub fn try_v1(&self) -> Result<&proc_macro::v1::Plugin> {
        self.plugin().as_v1().ok_or_else(|| {
            anyhow!(
                "procedural macro `{}` using v2 api used in a context expecting v1 api",
                self.package_id()
            )
        })
    }

    pub fn try_v2(&self) -> Result<&proc_macro::v2::Plugin> {
        self.plugin().as_v2().ok_or_else(|| {
            anyhow!(
                "procedural macro `{}` using v1 api used in a context expecting v2 api",
                self.package_id()
            )
        })
    }

    pub fn api_version(&self) -> ProcMacroApiVersion {
        self.plugin().api_version()
    }
}

/// This struct provides a unified interface for both v1 and v2 proc macro plugins.
///
/// It provides utilities for loading the macro implementation and invoking exposed interface,
/// but it should not implement any logic outside of macro api versioning.
pub enum VersionedPlugin {
    V1(proc_macro::v1::Plugin),
    V2(proc_macro::v2::Plugin),
}

impl VersionedPlugin {
    /// Load the shared library under the given path, and store versioned plugin instance.
    ///
    /// # Safety
    /// This function is unsafe because it calls the FFI interface of procedural macro package.
    pub unsafe fn try_new(package: &Package, lib_path: &Utf8Path) -> Result<Self> {
        let library = unsafe {
            SharedPluginLibrary::try_new(lib_path).with_context(|| {
                format!(
                    "failed to open dynamic library for `{}` proc macro",
                    package.id
                )
            })?
        };

        match library.api_version() {
            ProcMacroApiVersion::V1 => Ok(VersionedPlugin::V1(unsafe {
                proc_macro::v1::Plugin::try_new(library.into())?
            })),
            ProcMacroApiVersion::V2 => Ok(VersionedPlugin::V2(unsafe {
                proc_macro::v2::Plugin::try_new(library.into())?
            })),
        }
    }

    /// Obtain metadata of available expansions from the procedural macro.
    ///
    /// # Safety
    /// This function is unsafe because it calls the FFI interface of procedural macro package.
    pub unsafe fn load_expansions(&self, package_id: PackageId) -> Result<Vec<Expansion>> {
        match self {
            VersionedPlugin::V1(plugin) => unsafe { plugin.load_expansions(package_id) },
            VersionedPlugin::V2(plugin) => unsafe { plugin.load_expansions(package_id) },
        }
    }

    pub fn doc(&self, item_name: SmolStr) -> Option<String> {
        match self {
            VersionedPlugin::V1(plugin) => plugin.doc(item_name),
            VersionedPlugin::V2(plugin) => plugin.doc(item_name),
        }
    }

    pub fn as_v1(&self) -> Option<&proc_macro::v1::Plugin> {
        match self {
            VersionedPlugin::V1(plugin) => Some(plugin),
            _ => None,
        }
    }
    pub fn as_v2(&self) -> Option<&proc_macro::v2::Plugin> {
        match self {
            VersionedPlugin::V2(plugin) => Some(plugin),
            _ => None,
        }
    }

    pub fn api_version(&self) -> ProcMacroApiVersion {
        match self {
            VersionedPlugin::V1(_) => ProcMacroApiVersion::V1,
            VersionedPlugin::V2(_) => ProcMacroApiVersion::V2,
        }
    }
}
