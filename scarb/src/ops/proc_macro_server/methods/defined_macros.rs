use std::sync::Arc;

use anyhow::Result;
use scarb_proc_macro_server_types::methods::defined_macros::{
    DefinedMacros, DefinedMacrosResponse,
};

use super::Handler;
use crate::compiler::plugin::proc_macro::ProcMacroHost;

impl Handler for DefinedMacros {
    fn handle(
        proc_macro_host: Arc<ProcMacroHost>,
        _params: Self::Params,
    ) -> Result<Self::Response> {
        let mut response = proc_macro_host
            .macros()
            .iter()
            .map(|e| DefinedMacrosResponse {
                attributes: e.declared_attributes(),
                inline_macros: e.inline_macros(),
                derives: e.declared_derives(),
                executables: e.executable_attributes(),
            })
            .reduce(|mut acc, defined_macros| {
                acc.attributes.extend(defined_macros.attributes);
                acc.inline_macros.extend(defined_macros.inline_macros);
                acc.derives.extend(defined_macros.derives);
                acc.executables.extend(defined_macros.executables);

                acc
            })
            .unwrap_or_default();

        response.attributes.sort();
        response.attributes.dedup();

        response.inline_macros.sort();
        response.inline_macros.dedup();

        response.derives.sort();
        response.derives.dedup();

        response.executables.sort();
        response.executables.dedup();

        Ok(response)
    }
}
