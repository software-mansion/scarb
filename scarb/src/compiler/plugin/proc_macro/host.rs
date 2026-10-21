use crate::compiler::plugin::proc_macro::v2::ProcMacroId;
use crate::compiler::plugin::proc_macro::{ExpansionKind, ExpansionQuery, ProcMacroInstance};
use crate::compiler::plugin::{ProcMacroApiVersion, proc_macro};
use anyhow::Result;
use cairo_lang_semantic::db::SemanticGroup;
use cairo_lang_semantic::plugin::PluginSuite;
use itertools::Itertools;
use std::sync::Arc;

pub use scarb_proc_macro_host::FULL_PATH_MARKER_KEY;

/// The shared procedural macro host, driving the v2 dylib backend.
///
/// v1 macros keep their own host, as the shared one is built on the v2 macro api
/// (spanned token streams, `TextSpan` call sites). Moving v1 onto it would need conversions for
/// little gain, since v1 is nearly deprecated.
type V2HostPlugin = scarb_proc_macro_host::ProcMacroHostPlugin<proc_macro::v2::DylibBackend>;

pub enum ProcMacroHostPlugin {
    V1(Arc<proc_macro::v1::ProcMacroHostPlugin>),
    V2(Arc<V2HostPlugin>),
}

impl ProcMacroHostPlugin {
    pub fn new(instances: Vec<Arc<ProcMacroInstance>>, api_version: ProcMacroApiVersion) -> Self {
        assert!(
            instances
                .iter()
                .map(|instance| instance.api_version())
                .all_equal(),
            "all proc macro instances in a single host must use the same API version"
        );
        match api_version {
            ProcMacroApiVersion::V1 => Self::V1(Arc::new(
                proc_macro::v1::ProcMacroHostPlugin::new(instances),
            )),
            ProcMacroApiVersion::V2 => Self::V2(Arc::new(V2HostPlugin::new(Arc::new(
                proc_macro::v2::DylibBackend::new(instances),
            )))),
        }
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
            ProcMacroHostPlugin::V2(host) => V2HostPlugin::build_plugin_suite(host.clone()),
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

    /// Expansions of the given kind, each with the package providing it.
    fn expansions_with_package(&self, kind: ExpansionKind) -> Vec<ProcMacroId> {
        self.instances()
            .iter()
            .flat_map(|instance| {
                instance
                    .get_expansions()
                    .iter()
                    .filter(|expansion| expansion.kind == kind)
                    .map(|expansion| ProcMacroId::new(instance.package_id(), expansion.clone()))
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
