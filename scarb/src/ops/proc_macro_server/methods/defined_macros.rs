use std::sync::Arc;

use anyhow::Result;
use cairo_lang_utils::ordered_hash_map::OrderedHashMap;
use itertools::Itertools;
use scarb_proc_macro_server_types::methods::defined_macros::{
    DefinedMacros, DefinedMacrosCrateInfo, DefinedMacrosResponse,
};

use super::Handler;
use crate::compiler::plugin::proc_macro::ProcMacroHost;

impl Handler for DefinedMacros {
    fn handle(
        proc_macro_host: Arc<ProcMacroHost>,
        _params: Self::Params,
    ) -> Result<Self::Response> {
        let crate_macro_info = proc_macro_host
            .macros()
            .iter()
            .map(|macro_instance| {
                let attributes = macro_instance
                    .declared_attributes()
                    .into_iter()
                    .sorted()
                    .dedup()
                    .collect();

                let inline_macros = macro_instance
                    .inline_macros()
                    .into_iter()
                    .sorted()
                    .dedup()
                    .collect();

                let derives = macro_instance
                    .declared_derives()
                    .into_iter()
                    .sorted()
                    .dedup()
                    .collect();

                let executables = macro_instance
                    .executable_attributes()
                    .into_iter()
                    .sorted()
                    .dedup()
                    .collect();

                let package_name = macro_instance.package_id().name.to_smol_str();

                (
                    package_name,
                    DefinedMacrosCrateInfo {
                        attributes,
                        inline_macros,
                        derives,
                        executables,
                    },
                )
            })
            .collect::<OrderedHashMap<_, _>>();

        Ok(DefinedMacrosResponse { crate_macro_info })
    }
}
