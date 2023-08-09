use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use cairo_lang_defs::plugin::MacroPlugin;
use cairo_lang_starknet::plugin::StarkNetPlugin;
use itertools::Itertools;

use crate::compiler::plugin::builtin::BuiltinMacroCairoPlugin;
use crate::core::{PackageId, PackageName, SourceId};
use crate::internal::to_version::ToVersion;

pub mod builtin;

pub trait CairoPlugin: Sync {
    fn id(&self) -> PackageId;
    fn instantiate(&self) -> Result<Box<dyn CairoPluginInstance>>;
}

pub trait CairoPluginInstance {
    fn macro_plugins(&self) -> Vec<Arc<dyn MacroPlugin>>;
}

pub struct CairoPluginRepository {
    plugins: HashMap<PackageId, Box<dyn CairoPlugin>>,
}

impl CairoPluginRepository {
    pub fn empty() -> Self {
        Self {
            plugins: Default::default(),
        }
    }

    pub fn std() -> Self {
        let version = crate::version::get();
        let mut repo = Self::empty();

        // TODO(mkaput): Provide the plugin as `starknet_plugin` package and create regular
        //   `starknet` package which makes it a dependency. This way we can deliver Starknet Cairo
        //   library code to users etc.
        let starknet_package_id = PackageId::new(
            PackageName::STARKNET,
            version.cairo.version.to_version().unwrap(),
            SourceId::for_std(),
        );
        repo.add(Box::new(BuiltinMacroCairoPlugin::<StarkNetPlugin>::new(
            starknet_package_id,
        )))
        .unwrap();

        repo
    }

    pub fn add(&mut self, plugin: Box<dyn CairoPlugin>) -> Result<()> {
        match self.plugins.entry(plugin.id()) {
            Entry::Occupied(e) => bail!("found duplicate plugin `{}`", e.key()),
            Entry::Vacant(e) => {
                e.insert(plugin);
                Ok(())
            }
        }
    }

    pub fn get(&self, id: PackageId) -> Option<&dyn CairoPlugin> {
        self.plugins.get(&id).map(AsRef::as_ref)
    }

    pub fn fetch(&self, id: PackageId) -> Result<&dyn CairoPlugin> {
        self.get(id)
            .ok_or_else(|| anyhow!("compiler plugin could not be loaded `{id}`"))
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn CairoPlugin> {
        self.plugins.values().map(AsRef::as_ref)
    }
}

impl fmt::Debug for CairoPluginRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CompilerPluginRepository ")?;
        f.debug_set().entries(self.plugins.keys().sorted()).finish()
    }
}
