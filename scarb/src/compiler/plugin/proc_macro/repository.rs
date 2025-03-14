use crate::compiler::plugin::proc_macro::ProcMacroInstance;
use crate::compiler::plugin::proc_macro::compilation::SharedLibraryProvider;
use crate::core::{Config, Package, PackageId};
use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A global storage for dynamically-loaded procedural macros.
/// Loads dynamic shared libraries and hides them beside [`ProcMacroInstance`].
/// Guarantees that every library is loaded exactly once,
/// but does not prevent loading multiple versions of the same library.
#[derive(Default)]
pub struct ProcMacroRepository {
    /// A mapping between the [`PackageId`] of the package which defines the plugin
    /// and the [`ProcMacroInstance`] holding the underlying shared library.
    macros: RwLock<HashMap<PackageId, Arc<ProcMacroInstance>>>,
}

impl ProcMacroRepository {
    /// Returns the [`ProcMacroInstance`] representing the procedural macros defined in the [`Package`].
    /// Loads the underlying shared library if it has not been loaded yet.
    pub fn get_or_load(&self, package: Package, config: &Config) -> Result<Arc<ProcMacroInstance>> {
        let Ok(macros) = self.macros.read() else {
            bail!("could not get a read access to the ProcMacroRepository");
        };

        if let Some(instance) = macros.get(&package.id) {
            return Ok(instance.clone());
        }

        drop(macros);

        let Ok(mut macros) = self.macros.write() else {
            bail!("could not get a write access to the ProcMacroRepository");
        };

        let lib_path = package
            .shared_lib_path(config)
            .context("could not resolve shared library path")?;

        let instance = Arc::new(ProcMacroInstance::try_new(package.id, lib_path)?);
        macros.insert(package.id, instance.clone());

        Ok(instance)
    }
}
