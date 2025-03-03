use crate::compiler::plugin::{proc_macro_v1, proc_macro_v2, PluginApiVersion, ProcMacroInstance};
use anyhow::Result;
use cairo_lang_semantic::db::SemanticGroup;
use cairo_lang_semantic::plugin::PluginSuite;
use std::sync::Arc;

pub enum VersionedProcMacroHost {
    V1(Arc<proc_macro_v1::ProcMacroHostPlugin>),
    V2(Arc<proc_macro_v2::ProcMacroHostPlugin>),
}

impl VersionedProcMacroHost {
    pub fn try_new(
        instances: Vec<Arc<ProcMacroInstance>>,
        api_version: PluginApiVersion,
    ) -> Result<Self> {
        Ok(match api_version {
            PluginApiVersion::V1 => Self::V1(Arc::new(
                proc_macro_v1::ProcMacroHostPlugin::try_new(instances)?,
            )),
            PluginApiVersion::V2 => Self::V2(Arc::new(
                proc_macro_v2::ProcMacroHostPlugin::try_new(instances)?,
            )),
        })
    }

    pub fn macros(&self) -> &[Arc<ProcMacroInstance>] {
        match self {
            VersionedProcMacroHost::V1(host) => host.macros(),
            VersionedProcMacroHost::V2(host) => host.macros(),
        }
    }

    pub fn post_process(&self, db: &dyn SemanticGroup) -> Result<()> {
        match self {
            VersionedProcMacroHost::V1(host) => host.post_process(db),
            VersionedProcMacroHost::V2(host) => host.post_process(db),
        }
    }

    pub fn build_plugin_suite(&self) -> PluginSuite {
        match self {
            VersionedProcMacroHost::V1(host) => {
                proc_macro_v1::ProcMacroHostPlugin::build_plugin_suite(host.clone())
            }
            VersionedProcMacroHost::V2(host) => {
                proc_macro_v2::ProcMacroHostPlugin::build_plugin_suite(host.clone())
            }
        }
    }
}
