use std::sync::Arc;

use anyhow::{Context, Result};
use cairo_lang_macro_v2::{TextSpan, Token, TokenStream, TokenTree};
use scarb_proc_macro_server_types::methods::{expand::ExpandInline, ProcMacroResult};

use super::{from_v1_diagnostic, from_v2_token_stream, Handler};
use crate::compiler::plugin::proc_macro_common::VersionedPlugin;
use crate::compiler::plugin::{collection::WorkspaceProcMacros, proc_macro_common::ExpansionKind};

impl Handler for ExpandInline {
    fn handle(
        workspace_macros: Arc<WorkspaceProcMacros>,
        params: Self::Params,
    ) -> Result<Self::Response> {
        let Self::Params {
            context,
            name,
            args,
            call_site,
        } = params;

        let plugin = workspace_macros
            .get(&context.package_id)
            .with_context(|| format!("No macros found in scope: {context:?}"))?;

        let instance = plugin
            .macros()
            .iter()
            .find(|instance| {
                instance
                    .get_expansions()
                    .iter()
                    .filter(|expansion| expansion.kind == ExpansionKind::Inline)
                    .any(|expansion| expansion.name == name)
            })
            .with_context(|| format!("Unsupported inline macro: {name}"))?;

        let plugin = instance.plugin();
        let (token_stream, diagnostics) = match plugin {
            VersionedPlugin::V2(plugin) => {
                let result = plugin.generate_code(
                    name.into(),
                    call_site.clone(),
                    TokenStream::empty(),
                    args,
                );
                (result.token_stream, result.diagnostics)
            }
            VersionedPlugin::V1(plugin) => {
                let result = plugin.generate_code(
                    name.into(),
                    cairo_lang_macro::TokenStream::empty(),
                    from_v2_token_stream(args),
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

        Ok(ProcMacroResult {
            token_stream,
            diagnostics,
        })
    }
}
