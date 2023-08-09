use std::marker::PhantomData;
use std::sync::Arc;

use anyhow::Result;
use cairo_lang_defs::plugin::MacroPlugin;

use crate::compiler::plugin::{CairoPlugin, CairoPluginInstance};
use crate::core::PackageId;

pub struct BuiltinMacroCairoPlugin<P> {
    id: PackageId,
    phantom: PhantomData<P>,
}

impl<P> BuiltinMacroCairoPlugin<P> {
    pub fn new(id: PackageId) -> Self {
        Self {
            id,
            phantom: PhantomData,
        }
    }
}

impl<P: MacroPlugin + Default + 'static> CairoPlugin for BuiltinMacroCairoPlugin<P> {
    fn id(&self) -> PackageId {
        self.id
    }

    fn instantiate(&self) -> Result<Box<dyn CairoPluginInstance>> {
        let instance: Arc<dyn MacroPlugin> = Arc::new(P::default());
        Ok(Box::new(instance))
    }
}

impl CairoPluginInstance for Arc<dyn MacroPlugin> {
    fn macro_plugins(&self) -> Vec<Arc<dyn MacroPlugin>> {
        Vec::from_iter([self.clone()])
    }
}
