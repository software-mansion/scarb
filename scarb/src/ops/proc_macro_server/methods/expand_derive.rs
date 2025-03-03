use std::sync::Arc;

use anyhow::{Context, Result};
use cairo_lang_macro_v2::{TextSpan, Token, TokenStream, TokenTree};
use convert_case::{Case, Casing};
use scarb_proc_macro_server_types::methods::{expand::ExpandDerive, ProcMacroResult};

use super::{from_v1_diagnostic, from_v2_token_stream, Handler};
use crate::compiler::plugin::proc_macro_common::VersionedPlugin;
use crate::compiler::plugin::{
    collection::WorkspaceProcMacros,
    proc_macro_common::{Expansion, ExpansionKind},
};

impl Handler for ExpandDerive {
    fn handle(
        workspace_macros: Arc<WorkspaceProcMacros>,
        params: Self::Params,
    ) -> Result<Self::Response> {
        let Self::Params {
            context,
            derives,
            item,
            call_site,
        } = params;

        let mut derived_code = TokenStream::empty();
        let mut all_diagnostics = vec![];

        for derive in derives {
            let expansion = Expansion::new(derive.to_case(Case::Snake), ExpansionKind::Derive);

            let plugin = workspace_macros
                .get(&context.package_id)
                .with_context(|| format!("No macros found in scope {context:?}"))?;

            let instance = plugin
                .macros()
                .iter()
                .find(|instance| instance.get_expansions().contains(&expansion))
                .with_context(|| format!("Unsupported derive macro: {derive}"))?;

            let plugin = instance.plugin();
            let (token_stream, diagnostics) = match plugin {
                VersionedPlugin::V2(plugin) => {
                    let result = plugin.generate_code(
                        expansion.name.clone(),
                        call_site.clone(),
                        TokenStream::empty(),
                        item.clone(),
                    );
                    (result.token_stream, result.diagnostics)
                }
                VersionedPlugin::V1(plugin) => {
                    let result = plugin.generate_code(
                        expansion.name.clone(),
                        cairo_lang_macro::TokenStream::empty(),
                        from_v2_token_stream(item.clone()),
                    );
                    let token_stream = TokenStream::new(vec![TokenTree::Ident(Token::new(
                        result.token_stream.to_string(),
                        TextSpan::new(0, 0),
                    ))]);
                    let diagnostics = result
                        .diagnostics
                        .into_iter()
                        .map(from_v1_diagnostic)
                        .collect();
                    (token_stream, diagnostics)
                }
            };

            // Register diagnostics.
            all_diagnostics.extend(diagnostics);
            // Add generated code.
            derived_code.tokens.extend(token_stream.tokens);
        }

        Ok(ProcMacroResult {
            token_stream: derived_code,
            diagnostics: all_diagnostics,
        })
    }
}
