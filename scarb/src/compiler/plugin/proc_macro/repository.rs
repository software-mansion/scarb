use crate::compiler::CompilationUnitCairoPlugin;
use crate::compiler::plugin::proc_macro::ProcMacroInstance;
use crate::compiler::plugin::proc_macro::SharedLibraryProvider;
use crate::core::{Config, PackageId};
use anyhow::{Context, Result, bail, ensure};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A global storage for dynamically-loaded procedural macros.
/// Loads dynamic shared libraries and hides them beside [`ProcMacroInstance`].
/// Guarantees that every library is loaded exactly once,
/// but does not prevent loading multiple versions of the same library.
pub struct ProcMacroRepository {
    /// A mapping between the [`PackageId`] of the package which defines the plugin
    /// and the [`ProcMacroInstance`] holding the underlying shared library.
    macros: RwLock<HashMap<PackageId, Arc<ProcMacroInstance>>>,
    load_proc_macros: bool,
}

impl ProcMacroRepository {
    pub fn new(load_proc_macros: bool) -> Self {
        Self {
            macros: Default::default(),
            load_proc_macros,
        }
    }

    /// Returns the [`ProcMacroInstance`] representing the procedural macros defined in the [`CompilationUnitCairoPlugin`].
    /// Loads the underlying shared library if it has not been loaded yet.
    pub fn get_or_load(
        &self,
        plugin: &CompilationUnitCairoPlugin,
        config: &Config,
    ) -> Result<Arc<ProcMacroInstance>> {
        ensure!(
            self.load_proc_macros,
            "procedural macros are disallowed with `--no-proc-macros` flag"
        );

        let Ok(macros) = self.macros.read() else {
            bail!("could not get a read access to the ProcMacroRepository");
        };

        if let Some(instance) = macros.get(&plugin.package.id) {
            return Ok(instance.clone());
        }

        drop(macros);

        let Ok(mut macros) = self.macros.write() else {
            bail!("could not get a write access to the ProcMacroRepository");
        };

        let lib_path = plugin
            .shared_lib_path(config)
            .context("could not resolve shared library path")?;

        let instance = Arc::new(ProcMacroInstance::try_new(&plugin.package, lib_path)?);
        macros.insert(plugin.package.id, instance.clone());

        Ok(instance)
    }

    pub fn load_proc_macros(&self) -> bool {
        self.load_proc_macros
    }
}
