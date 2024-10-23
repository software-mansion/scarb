use crate::{
    compiler::plugin::proc_macro::ProcMacroHost, ops::proc_macro_server::json_rpc::Handler,
};
use anyhow::Result;
use proc_macro_server_api::methods::defined_macros::{DefinedMacros, DefinedMacrosResponse};
use std::sync::Arc;

impl Handler for DefinedMacros {
    fn handle(proc_macros: Arc<ProcMacroHost>, _params: Self::Params) -> Result<Self::Response> {
        let mut response: DefinedMacrosResponse = proc_macros
            .macros()
            .into_iter()
            .map(|e| DefinedMacrosResponse {
                attributes: e.declared_attributes(),
                inline_macros: e.inline_macros(),
                derives: e.declared_derives(),
                executables: e.executable_attributes(),
            })
            .sum();

        response
            .attributes
            .retain(|attr| !response.executables.contains(attr));

        Ok(response)
    }
}
