use std::marker::PhantomData;
use std::sync::Arc;

use cairo_lang_semantic::plugin::SemanticPlugin;

use crate::compiler::plugin::{CairoPlugin, CairoPluginInstance};
use crate::core::PackageId;

pub struct BuiltinSemanticCairoPlugin<P> {
    id: PackageId,
    phantom: PhantomData<P>,
}

impl<P> BuiltinSemanticCairoPlugin<P> {
    pub fn new(id: PackageId) -> Self {
        Self {
            id,
            phantom: PhantomData,
        }
    }
}

impl<P: SemanticPlugin + Default + 'static> CairoPlugin for BuiltinSemanticCairoPlugin<P> {
    fn id(&self) -> PackageId {
        self.id
    }

    fn instantiate(&self) -> anyhow::Result<Box<dyn CairoPluginInstance>> {
        let instance: Arc<dyn SemanticPlugin> = Arc::new(P::default());
        Ok(Box::new(instance))
    }
}

impl CairoPluginInstance for Arc<dyn SemanticPlugin> {
    fn semantic_plugins(&self) -> Vec<Arc<dyn SemanticPlugin>> {
        Vec::from_iter([self.clone()])
    }
}
