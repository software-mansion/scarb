use crate::compiler::CompilationUnitCairoPlugin;
use crate::compiler::plugin::proc_macro::{
    Expansion, ExpansionKind, ProcMacroApiVersion, ProcMacroInstance,
};
use crate::core::Config;
use anyhow::{Result, anyhow};
use camino::Utf8Path;
use convert_case::{Case, Casing};
use libloading::Library;
use smol_str::ToSmolStr;
use std::ffi::{CStr, c_char};
use std::num::NonZeroU8;
use std::sync::Arc;
use tracing::debug;

/// This constant is used to identify executable attributes.
///
/// An attribute is considered executable if it starts with this prefix.
/// No other metadata is stored for executable attributes.
/// This means, that this constant is part of the stable contract between Scarb and procedural macro.
/// Warning: Changing this would be breaking to existing macros!
const EXEC_ATTR_PREFIX: &str = "__exec_attr_";

pub struct SharedPluginLibrary {
    api_version: ProcMacroApiVersion,
    library: Library,
}

impl SharedPluginLibrary {
    /// Load the shared library under the given path, and store its version.
    ///
    /// # Safety
    /// This function is unsafe because it calls the FFI interface of procedural macro package.
    pub unsafe fn try_new(lib_path: &Utf8Path) -> Result<Self> {
        let library = unsafe { Library::new(lib_path.as_std_path())? };
        let api_version: ProcMacroApiVersion = if let Ok(symbol) =
            unsafe { library.get::<*mut NonZeroU8>(b"CAIRO_LANG_MACRO_API_VERSION\0") }
        {
            let api_version: NonZeroU8 = unsafe { **symbol };
            let api_version: u8 = api_version.get();
            api_version.try_into()?
        } else {
            debug!(
                "CAIRO_LANG_MACRO_API_VERSION symbol for `{}` proc macro not found, defaulting to V1 API version",
                lib_path
            );
            ProcMacroApiVersion::V1
        };
        Ok(Self {
            library,
            api_version,
        })
    }

    pub fn api_version(&self) -> ProcMacroApiVersion {
        self.api_version
    }
}

impl TryFrom<u8> for ProcMacroApiVersion {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(ProcMacroApiVersion::V1),
            2 => Ok(ProcMacroApiVersion::V2),
            _ => Err(anyhow!(
                "unsupported proc macro api version `{}`, expected `1` or `2`",
                value
            )),
        }
    }
}

impl From<SharedPluginLibrary> for Library {
    fn from(plugin: SharedPluginLibrary) -> Self {
        plugin.library
    }
}

pub trait InstanceLoader {
    fn instantiate(&self, config: &Config) -> Result<Arc<ProcMacroInstance>>;
}

impl InstanceLoader for CompilationUnitCairoPlugin {
    fn instantiate(&self, config: &Config) -> Result<Arc<ProcMacroInstance>> {
        self.prebuilt
            .clone()
            .map(Ok)
            .unwrap_or_else(|| config.proc_macro_repository().get_or_load(self, config))
    }
}

/// Builds an expansion from its name and kind, as exported by either procedural macro api version.
///
/// # Safety
/// `name` must be null or point to a valid C string. This does not take ownership of it.
pub unsafe fn expansion_from_stable(name: *const c_char, kind: ExpansionKind) -> Expansion {
    let name = if name.is_null() {
        String::default()
    } else {
        let cstr = unsafe { CStr::from_ptr(name) };
        cstr.to_string_lossy().to_string()
    };
    // Handle special case for executable attributes.
    if let Some(name) = name.strip_prefix(EXEC_ATTR_PREFIX) {
        let name = name.to_smolstr();
        return Expansion {
            cairo_name: name.clone(),
            expansion_name: name,
            kind: ExpansionKind::Executable,
        };
    }
    let cairo_name = if matches!(kind, ExpansionKind::Derive) {
        name.to_case(Case::UpperCamel).to_smolstr()
    } else {
        name.to_smolstr()
    };
    Expansion {
        cairo_name,
        expansion_name: name.to_smolstr(),
        kind,
    }
}
