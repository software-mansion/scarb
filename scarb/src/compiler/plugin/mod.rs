use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;

use crate::compiler::plugin::builtin::BuiltinTestAssertsPlugin;
use anyhow::{anyhow, bail, Result};
use cairo_lang_semantic::plugin::PluginSuite;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::compiler::plugin::builtin::BuiltinCairoRunPlugin;
use crate::core::{Package, PackageId, TargetKind, Workspace};

use self::builtin::{BuiltinStarkNetPlugin, BuiltinTestPlugin};

pub mod builtin;
pub mod proc_macro;

/// Properties that can be defined on Cairo plugin target.
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct CairoPluginProps {
    /// Mark this macro plugin as builtin.
    /// Builtin plugins are assumed to be available in `CairoPluginRepository` for the whole Scarb execution.
    pub builtin: bool,
}

pub fn fetch_cairo_plugin(package: &Package, ws: &Workspace<'_>) -> Result<()> {
    assert!(package.is_cairo_plugin());
    let target = package.fetch_target(&TargetKind::CAIRO_PLUGIN)?;
    let props: CairoPluginProps = target.props()?;
    // No need to fetch for buildin plugins.
    if !props.builtin {
        proc_macro::fetch_crate(package, ws)?;
    }
    Ok(())
}

pub trait CairoPlugin: Sync {
    fn id(&self) -> PackageId;
    fn instantiate(&self) -> Result<Box<dyn CairoPluginInstance>>;
}

pub trait CairoPluginInstance {
    fn plugin_suite(&self) -> PluginSuite;
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
        let mut repo = Self::empty();

        // TODO(mkaput): Provide the plugin as `starknet_plugin` package and create regular
        //   `starknet` package which makes it a dependency. This way we can deliver Starknet Cairo
        //   library code to users etc.
        repo.add(Box::new(BuiltinStarkNetPlugin)).unwrap();
        repo.add(Box::new(BuiltinTestPlugin)).unwrap();
        repo.add(Box::new(BuiltinCairoRunPlugin)).unwrap();
        repo.add(Box::new(BuiltinTestAssertsPlugin)).unwrap();
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
