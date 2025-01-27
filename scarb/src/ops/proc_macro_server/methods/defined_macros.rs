use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use cairo_lang_defs::plugin::MacroPlugin;
use scarb_proc_macro_server_types::methods::defined_macros::{
    CompilationUnitDefinedMacrosInfo, DefinedMacros, DefinedMacrosResponse,
};

use super::Handler;
use crate::compiler::plugin::collection::WorkspaceProcMacros;

impl Handler for DefinedMacros {
    fn handle(
        workspace_macros: Arc<WorkspaceProcMacros>,
        _params: Self::Params,
    ) -> Result<Self::Response> {
        let workspace_macro_info = workspace_macros
            .macros_for_compilation_units
            .iter()
            .map(|(cu_main_component_id, plugin)| {
                let attributes = plugin.declared_attributes();
                let inline_macros = plugin.declared_inline_macros();
                let derives = plugin.declared_derives();
                let executables = plugin.executable_attributes();

                (
                    cu_main_component_id.to_owned(),
                    CompilationUnitDefinedMacrosInfo {
                        attributes,
                        inline_macros,
                        derives,
                        executables,
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        Ok(DefinedMacrosResponse {
            workspace_macro_info,
        })
    }
}
