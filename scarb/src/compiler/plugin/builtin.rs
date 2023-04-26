use std::marker::PhantomData;
use std::sync::Arc;

use cairo_lang_semantic::plugin::SemanticPlugin;

use crate::compiler::plugin::{CompilerPlugin, CompilerPluginInstance};
use crate::core::PackageId;

pub struct BuiltinSemanticCompilerPlugin<P> {
    id: PackageId,
    phantom: PhantomData<P>,
}

impl<P> BuiltinSemanticCompilerPlugin<P> {
    pub fn new(id: PackageId) -> Self {
        Self {
            id,
            phantom: PhantomData,
        }
    }
}

impl<P: SemanticPlugin + Default + 'static> CompilerPlugin for BuiltinSemanticCompilerPlugin<P> {
    fn id(&self) -> PackageId {
        self.id
    }

    fn instantiate(&self) -> anyhow::Result<Box<dyn CompilerPluginInstance>> {
        let instance: Arc<dyn SemanticPlugin> = Arc::new(P::default());
        Ok(Box::new(instance))
    }
}

impl CompilerPluginInstance for Arc<dyn SemanticPlugin> {
    fn semantic_plugins(&self) -> Vec<Arc<dyn SemanticPlugin>> {
        Vec::from_iter([self.clone()])
    }
}
