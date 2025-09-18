use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use itertools::Itertools;
use scarb_proc_macro_server_types::methods::defined_macros::{
    CompilationUnitComponentMacros, DebugInfo, DefinedMacros, DefinedMacrosResponse,
};
use tracing::trace;

use super::Handler;
use crate::compiler::plugin::proc_macro::DeclaredProcMacroInstances;
use crate::ops::store::ProcMacroStore;

impl Handler for DefinedMacros {
    fn handle(
        workspace_macros: Arc<Mutex<ProcMacroStore>>,
        params: Self::Params,
    ) -> Result<Self::Response> {
        let Self::Params { workspace } = params;

        trace!("[PMS] Handling definedMacros");

        let macros_for_cu_components = workspace_macros
            .lock()
            .unwrap()
            .get_workspace_macros(&workspace)
            .with_context(|| format!("workspace {workspace:?} not found"))?
            .macros_for_components
            .iter()
            .flat_map(|(component, plugin)| {
                plugin
                    .iter()
                    .map(|plugin| {
                        let attributes = plugin.declared_attributes_without_executables();
                        let inline_macros = plugin.declared_inline_macros();
                        let derives = plugin.declared_derives_snake_case();
                        let executables = plugin.executable_attributes();
                        let source_packages = plugin
                            .instances()
                            .iter()
                            .map(|instance| instance.package_id().to_serialized_string())
                            .collect();

                        CompilationUnitComponentMacros {
                            component: component.to_owned(),
                            attributes,
                            inline_macros,
                            derives,
                            executables,
                            debug_info: DebugInfo { source_packages },
                        }
                    })
                    .collect_vec()
            })
            .map(|cu_components_macros| {
                (cu_components_macros.component.clone(), cu_components_macros)
            })
            .into_group_map()
            .iter()
            .map(|(component, cu_macros)| {
                let mut derives = Vec::new();
                let mut executables = Vec::new();
                let mut attributes = Vec::new();
                let mut inline_macros = Vec::new();
                let mut source_packages = Vec::new();

                for macros in cu_macros {
                    derives.extend(macros.derives.clone());
                    executables.extend(macros.executables.clone());
                    attributes.extend(macros.attributes.clone());
                    inline_macros.extend(macros.inline_macros.clone());
                    source_packages.extend(macros.debug_info.source_packages.clone());
                }

                CompilationUnitComponentMacros {
                    component: component.clone(),
                    executables,
                    derives,
                    attributes,
                    inline_macros,
                    debug_info: DebugInfo { source_packages },
                }
            })
            .collect();

        trace!("[PMS] Sending definedMacros response");

        Ok(DefinedMacrosResponse {
            macros_for_cu_components,
        })
    }
}
