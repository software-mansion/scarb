use std::sync::Arc;

use anyhow::Result;
use convert_case::{Case, Casing};
use itertools::Itertools;
use scarb_proc_macro_server_types::methods::defined_macros::{
    CompilationUnitComponentMacros, DefinedMacros, DefinedMacrosResponse,
};

use super::Handler;
use crate::compiler::plugin::collection::WorkspaceProcMacros;
use crate::compiler::plugin::proc_macro::DeclaredProcMacroInstances;

impl Handler for DefinedMacros {
    fn handle(
        workspace_macros: Arc<WorkspaceProcMacros>,
        _params: Self::Params,
    ) -> Result<Self::Response> {
        let macros_for_cu_components = workspace_macros
            .macros_for_components
            .iter()
            .flat_map(|(component, plugin)| {
                plugin
                    .iter()
                    .map(|plugin| {
                        let attributes = plugin.declared_attributes_without_executables();
                        let inline_macros = plugin.declared_inline_macros();
                        let derives = plugin
                            .declared_derives()
                            .into_iter()
                            .map(|name| name.to_case(Case::Snake))
                            .collect();
                        let executables = plugin.executable_attributes();

                        CompilationUnitComponentMacros {
                            component: component.to_owned(),
                            attributes,
                            inline_macros,
                            derives,
                            executables,
                        }
                    })
                    .collect_vec()
            })
            .collect();

        Ok(DefinedMacrosResponse {
            macros_for_cu_components,
        })
    }
}
