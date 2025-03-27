use crate::compiler::plugin::proc_macro::ProcMacroApiVersion;
use anyhow::{Result, anyhow};
use camino::Utf8Path;
use libloading::Library;
use std::num::NonZeroU8;
use tracing::debug;

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
        let library = unsafe { Library::new(lib_path)? };
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
