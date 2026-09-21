mod span_adapter;

use std::sync::{Arc, OnceLock};

use cairo_lang_defs::plugin::{
    InlineMacroExprPlugin, InlinePluginResult, MacroPluginMetadata, PluginGeneratedFile,
    PluginResult,
};
use cairo_lang_macro::{
    AllocationContext, ProcMacroResult, TextSpan, TokenStream, TokenStreamMetadata,
};
use cairo_lang_syntax::node::ast::PathSegment;
use cairo_lang_syntax::node::{Terminal, TypedSyntaxNode, ast};
use salsa::Database;

use crate::backend::{ExpansionId, ProcMacroBackend};
use crate::conversion::{CallSiteLocation, into_cairo_diagnostics};
use crate::expansion::{ExpansionKind, ExpansionQuery};
use crate::host::inline::span_adapter::InlineAdapter;
use crate::host::{ProcMacroHostPlugin, generate_code_mappings};
use crate::token_stream_builder::TokenStreamBuilder;

/// A Cairo compiler inline macro plugin controlling the inline procedural macro execution.
///
/// This plugin represents a single expansion capable of handling inline procedural macros.
/// The plugin triggers code expansion in a corresponding procedural macro instance.
#[derive(Debug)]
pub struct ProcMacroInlinePlugin<B: ProcMacroBackend> {
    backend: Arc<B>,
    id: B::Id,
    doc: OnceLock<Option<String>>,
}

impl<B: ProcMacroBackend> ProcMacroInlinePlugin<B> {
    pub fn new(backend: Arc<B>, id: B::Id) -> Self {
        Self {
            backend,
            id,
            doc: Default::default(),
        }
    }

    pub fn backend(&self) -> &Arc<B> {
        &self.backend
    }

    fn expand(
        &self,
        db: &dyn Database,
        call_site: TextSpan,
        args: TokenStream,
        item: TokenStream,
        aux_data: &mut B::AuxData,
    ) -> ProcMacroResult {
        let result = self.backend.expand(db, &self.id, call_site, args, item);
        self.backend.on_expanded(&self.id, &result, aux_data);
        result
    }
}

impl<B: ProcMacroBackend> InlineMacroExprPlugin for ProcMacroInlinePlugin<B> {
    #[tracing::instrument(level = "trace", skip_all)]
    fn generate_code<'db>(
        &self,
        db: &'db dyn Database,
        syntax: &ast::ExprInlineMacro<'db>,
        _metadata: &MacroPluginMetadata<'_>,
    ) -> InlinePluginResult<'db> {
        let call_site = CallSiteLocation::new(syntax, db);
        let ctx = AllocationContext::default();
        let arguments = syntax.arguments(db);
        let mut token_stream_builder = TokenStreamBuilder::new(db);
        token_stream_builder.add_node(arguments.as_syntax_node());
        let token_stream = token_stream_builder.build(&ctx);
        let (adapter, adapted_token_stream) = InlineAdapter::adapt_token_stream(
            token_stream,
            arguments.as_syntax_node().span(db),
            call_site.span.clone(),
        );
        let adapted_call_site = adapter.adapted_call_site();
        let mut aux_data = B::AuxData::default();
        let result = self.expand(
            db,
            adapted_call_site.clone(),
            TokenStream::empty(),
            adapted_token_stream,
            &mut aux_data,
        );
        // Handle diagnostics.
        let diagnostics = into_cairo_diagnostics(
            db,
            adapter.adapt_diagnostics(result.diagnostics),
            call_site.stable_ptr,
        );
        let token_stream = result.token_stream.clone();
        if token_stream.is_empty() {
            // Remove original code
            InlinePluginResult {
                code: None,
                diagnostics,
            }
        } else {
            // Replace
            let aux_data = self.backend.finish_aux_data(aux_data);
            let content = token_stream.to_string();
            let code_mappings = adapter.adapt_code_mappings(generate_code_mappings(
                &token_stream,
                adapted_call_site.clone(),
            ));
            InlinePluginResult {
                code: Some(PluginGeneratedFile {
                    name: "inline_proc_macro".into(),
                    code_mappings,
                    content,
                    aux_data,
                    diagnostics_note: Some(format!(
                        "this error originates in the inline macro: `{}`",
                        self.id.expansion().cairo_name
                    )),
                    is_unhygienic: false,
                }),
                diagnostics,
            }
        }
    }

    fn documentation(&self) -> Option<String> {
        self.doc.get_or_init(|| self.backend.doc(&self.id)).clone()
    }
}

#[tracing::instrument(level = "trace", skip_all)]
pub(crate) fn expand_module_level_inline_macro<'db, B: ProcMacroBackend>(
    host: &ProcMacroHostPlugin<B>,
    db: &'db dyn Database,
    inline_macro: &ast::ItemInlineMacro<'db>,
    _metadata: &TokenStreamMetadata,
) -> Option<PluginResult<'db>> {
    let path = inline_macro.path(db).segments(db).elements(db).last()?;
    let PathSegment::Simple(segment) = path else {
        return None;
    };
    let value = segment.ident(db).text(db).to_string(db);
    let found = host.find_expansion(&ExpansionQuery::with_cairo_name(
        &value,
        ExpansionKind::Inline,
    ))?;

    let call_site = CallSiteLocation::new(inline_macro, db);
    let ctx = AllocationContext::default();
    let arguments = inline_macro.arguments(db);

    let mut token_stream_builder = TokenStreamBuilder::new(db);
    token_stream_builder.add_node(arguments.as_syntax_node());
    let token_stream = token_stream_builder.build(&ctx);

    let (adapter, adapted_token_stream) = InlineAdapter::adapt_token_stream(
        token_stream,
        arguments.as_syntax_node().span(db),
        call_site.span.clone(),
    );
    let adapted_call_site = adapter.adapted_call_site();

    let mut aux_data = B::AuxData::default();
    let result = host.expand(
        db,
        &found,
        adapted_call_site.clone(),
        TokenStream::empty(),
        adapted_token_stream,
        &mut aux_data,
    );

    let diagnostics = into_cairo_diagnostics(
        db,
        adapter.adapt_diagnostics(result.diagnostics.clone()),
        call_site.stable_ptr,
    );

    let token_stream = result.token_stream.clone();
    if token_stream.is_empty() {
        // Remove original code
        return Some(PluginResult {
            code: None,
            diagnostics,
            remove_original_item: true,
        });
    }

    let aux_data = host.backend().finish_aux_data(aux_data);
    let code_mappings = adapter.adapt_code_mappings(generate_code_mappings(
        &result.token_stream,
        adapted_call_site.clone(),
    ));

    Some(PluginResult {
        code: Some(PluginGeneratedFile {
            name: "inline_proc_macro".into(),
            code_mappings,
            content: token_stream.to_string(),
            aux_data,
            diagnostics_note: Some(format!(
                "this error originates in the inline macro: `{}`",
                found.expansion().cairo_name
            )),
            is_unhygienic: false,
        }),
        diagnostics,
        remove_original_item: true,
    })
}
