use crate::compiler::plugin::proc_macro::{
    Expansion, ExpansionKind, ExpansionQuery, ProcMacroInstance,
};
use crate::compiler::plugin::{ProcMacroApiVersion, proc_macro};
use crate::core::PackageId;
use anyhow::Result;
use cairo_lang_semantic::db::SemanticGroup;
use cairo_lang_semantic::plugin::PluginSuite;
use itertools::Itertools;
use std::sync::Arc;

pub use scarb_proc_macro_host::FULL_PATH_MARKER_KEY;

pub enum ProcMacroHostPlugin {
    V1(Arc<proc_macro::v1::ProcMacroHostPlugin>),
    V2(Arc<proc_macro::v2::ProcMacroHostPlugin>),
}

impl ProcMacroHostPlugin {
    pub fn try_new(
        instances: Vec<Arc<ProcMacroInstance>>,
        api_version: ProcMacroApiVersion,
    ) -> Result<Self> {
        assert!(
            instances
                .iter()
                .map(|instance| instance.api_version())
                .all_equal(),
            "all proc macro instances in a single host must use the same API version"
        );
        Ok(match api_version {
            ProcMacroApiVersion::V1 => Self::V1(Arc::new(
                proc_macro::v1::ProcMacroHostPlugin::try_new(instances)?,
            )),
            ProcMacroApiVersion::V2 => {
                Self::V2(Arc::new(proc_macro::v2::ProcMacroHostPlugin::new(
                    Arc::new(proc_macro::v2::DylibBackend::try_new(instances)?),
                )))
            }
        })
    }

    pub fn post_process(&self, db: &dyn SemanticGroup) -> Result<()> {
        match self {
            ProcMacroHostPlugin::V1(host) => host.post_process(db),
            ProcMacroHostPlugin::V2(host) => host.backend().post_process(db),
        }
    }

    pub fn build_plugin_suite(&self) -> PluginSuite {
        match self {
            ProcMacroHostPlugin::V1(host) => {
                proc_macro::v1::ProcMacroHostPlugin::build_plugin_suite(host.clone())
            }
            ProcMacroHostPlugin::V2(host) => {
                proc_macro::v2::ProcMacroHostPlugin::build_plugin_suite(host.clone())
            }
        }
    }

    pub fn api_version(&self) -> ProcMacroApiVersion {
        match self {
            ProcMacroHostPlugin::V1(_) => ProcMacroApiVersion::V1,
            ProcMacroHostPlugin::V2(_) => ProcMacroApiVersion::V2,
        }
    }
}

pub trait DeclaredProcMacroInstances {
    fn instances(&self) -> &[Arc<ProcMacroInstance>];

    fn find_instance_with_expansion(
        &self,
        expansion: &ExpansionQuery,
    ) -> Option<&Arc<ProcMacroInstance>> {
        self.instances().iter().find(|instance| {
            instance
                .get_expansions()
                .iter()
                .any(|exp| exp.matches_query(expansion))
        })
    }

    fn declared_inline_macros(&self) -> Vec<String> {
        self.instances()
            .iter()
            .flat_map(|instance| instance.inline_macros())
            .collect()
    }

    fn declared_derives(&self) -> Vec<String> {
        self.instances()
            .iter()
            .flat_map(|m| m.declared_derives())
            .collect()
    }

    fn executable_attributes(&self) -> Vec<String> {
        self.instances()
            .iter()
            .flat_map(|m| m.executable_attributes())
            .collect()
    }

    fn declared_attributes(&self) -> Vec<String> {
        self.instances()
            .iter()
            .flat_map(|m| m.declared_attributes_and_executables())
            .chain(vec![FULL_PATH_MARKER_KEY.to_string()])
            .collect()
    }

    /// Expansions of the given kind, each with the package of the macro providing it.
    ///
    /// Used by the proc macro server to describe the available macros to the language server.
    fn expansions_with_package(&self, kind: ExpansionKind) -> Vec<(Expansion, PackageId)> {
        self.instances()
            .iter()
            .flat_map(|instance| {
                instance
                    .get_expansions()
                    .iter()
                    .filter(|expansion| expansion.kind == kind)
                    .map(|expansion| (expansion.clone(), instance.package_id()))
            })
            .collect()
    }
}

impl DeclaredProcMacroInstances for ProcMacroHostPlugin {
    fn instances(&self) -> &[Arc<ProcMacroInstance>] {
        match self {
            ProcMacroHostPlugin::V1(host) => host.instances(),
            ProcMacroHostPlugin::V2(host) => host.backend().instances(),
        }
    }
}
