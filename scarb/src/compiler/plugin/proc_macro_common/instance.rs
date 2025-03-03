use super::expansion::Expansion;
use crate::compiler::plugin::proc_macro_common::shared_lib_provider::SharedLibraryProvider;
use crate::compiler::plugin::proc_macro_common::ExpansionKind;
use crate::compiler::plugin::{proc_macro_v1, proc_macro_v2, CairoPluginProps, PluginApiVersion};
use crate::core::{Package, PackageId, TargetKind};
use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use smol_str::SmolStr;
use std::fmt::Debug;
use tracing::trace;

/// Representation of a single procedural macro.
///
/// This struct is a wrapper around a shared library containing the procedural macro implementation.
/// It is responsible for loading the shared library and providing a safe interface for code expansion.
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
pub enum VersionedPlugin {
    V1(proc_macro_v1::Plugin),
    V2(proc_macro_v2::Plugin),
}

impl VersionedPlugin {
    pub fn try_new(package: &Package, lib_path: &Utf8Path) -> Result<Self> {
        let api = plugin_api_version(package)?;
        let plugin = if let PluginApiVersion::V1 = api {
            VersionedPlugin::V1(unsafe { proc_macro_v1::Plugin::try_new(lib_path)? })
        } else {
            VersionedPlugin::V2(unsafe { proc_macro_v2::Plugin::try_new(lib_path)? })
        };
        Ok(plugin)
    }
    ///
    /// # Safety
    pub unsafe fn load_expansions(&self, package_id: PackageId) -> Result<Vec<Expansion>> {
        match self {
            VersionedPlugin::V1(plugin) => plugin.load_expansions(package_id),
            VersionedPlugin::V2(plugin) => plugin.load_expansions(package_id),
        }
    }
    pub fn doc(&self, item_name: SmolStr) -> Option<String> {
        match self {
            VersionedPlugin::V1(plugin) => plugin.doc(item_name),
            VersionedPlugin::V2(plugin) => plugin.doc(item_name),
        }
    }
    pub fn as_v1(&self) -> Option<&proc_macro_v1::Plugin> {
        match self {
            VersionedPlugin::V1(plugin) => Some(plugin),
            _ => None,
        }
    }
    pub fn as_v2(&self) -> Option<&proc_macro_v2::Plugin> {
        match self {
            VersionedPlugin::V2(plugin) => Some(plugin),
            _ => None,
        }
    }
    pub fn api_version(&self) -> PluginApiVersion {
        match self {
            VersionedPlugin::V1(_) => PluginApiVersion::V1,
            VersionedPlugin::V2(_) => PluginApiVersion::V2,
        }
    }
}

fn plugin_api_version(package: &Package) -> Result<PluginApiVersion> {
    assert!(package.is_cairo_plugin());
    let target = package.fetch_target(&TargetKind::CAIRO_PLUGIN)?;
    let props: CairoPluginProps = target.props()?;
    Ok(props.api)
}

impl ProcMacroInstance {
    /// Load shared library
    pub fn try_new(package: &Package, lib_path: Utf8PathBuf) -> Result<Self> {
        trace!("loading compiled macro for `{}` package", package.id);
        let plugin = VersionedPlugin::try_new(package, &lib_path)?;
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
        let plugin = VersionedPlugin::try_new(&package, &prebuilt_path)?;
        Ok(Self {
            expansions: unsafe { plugin.load_expansions(package.id)? },
            package_id: package.id,
            plugin,
        })
    }

    pub fn get_expansions(&self) -> &[Expansion] {
        &self.expansions
    }

    pub fn package_id(&self) -> PackageId {
        self.package_id
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
        self.plugin.doc(item_name)
    }
    pub fn plugin(&self) -> &VersionedPlugin {
        &self.plugin
    }
    pub fn api_version(&self) -> PluginApiVersion {
        self.plugin().api_version()
    }
}
