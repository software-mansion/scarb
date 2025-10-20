mod span_adapter;

use crate::compiler::plugin::proc_macro::ProcMacroInstance;
use crate::compiler::plugin::proc_macro::expansion::{Expansion, ExpansionKind, ExpansionQuery};
use crate::compiler::plugin::proc_macro::v2::host::TokenStreamMetadata;
use crate::compiler::plugin::proc_macro::v2::host::aux_data::{EmittedAuxData, ProcMacroAuxData};
use crate::compiler::plugin::proc_macro::v2::host::conversion::{
    CallSiteLocation, into_cairo_diagnostics,
};
use crate::compiler::plugin::proc_macro::v2::host::generate_code_mappings;
use crate::compiler::plugin::proc_macro::v2::host::inline::span_adapter::InlineAdapter;
use crate::compiler::plugin::proc_macro::v2::{
    ProcMacroHostPlugin, ProcMacroId, TokenStreamBuilder,
};
use cairo_lang_defs::plugin::{
    DynGeneratedFileAuxData, InlineMacroExprPlugin, InlinePluginResult, MacroPluginMetadata,
    PluginGeneratedFile, PluginResult,
};
use cairo_lang_macro::{AllocationContext, TokenStream};
use cairo_lang_syntax::node::ast::PathSegment;
use cairo_lang_syntax::node::{Terminal, TypedSyntaxNode, ast};
use salsa::Database;
use std::sync::{Arc, OnceLock};

/// A Cairo compiler inline macro plugin controlling the inline procedural macro execution.
///
/// This plugin represents a single expansion capable of handling inline procedural macros.
/// The plugin triggers code expansion in a corresponding procedural macro instance.
#[derive(Debug)]
pub struct ProcMacroInlinePlugin {
    instance: Arc<ProcMacroInstance>,
    expansion: Expansion,
    doc: OnceLock<Option<String>>,
}

impl ProcMacroInlinePlugin {
    pub fn new(instance: Arc<ProcMacroInstance>, expansion: Expansion) -> Self {
        assert!(instance.get_expansions().contains(&expansion));
        Self {
            instance,
            expansion,
            doc: Default::default(),
        }
    }

    fn instance(&self) -> &ProcMacroInstance {
        &self.instance
    }
}

impl InlineMacroExprPlugin for ProcMacroInlinePlugin {
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
        let result = self
            .instance()
            .try_v2()
            .expect("procedural macro using v1 api used in a context expecting v2 api")
            .generate_code(
                self.expansion.expansion_name.clone(),
                adapted_call_site.clone(),
                TokenStream::empty(),
                adapted_token_stream,
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
            let aux_data = result.aux_data.map(|aux_data| {
                let aux_data = ProcMacroAuxData::new(
                    aux_data.into(),
                    ProcMacroId::new(self.instance.package_id(), self.expansion.clone()),
                );
                let mut emitted = EmittedAuxData::default();
                emitted.push(aux_data);
                DynGeneratedFileAuxData::new(emitted)
            });
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
                        self.expansion.cairo_name
                    )),
                    is_unhygienic: false,
                }),
                diagnostics,
            }
        }
    }

    fn documentation(&self) -> Option<String> {
        self.doc
            .get_or_init(|| self.instance().doc(self.expansion.cairo_name.clone()))
            .clone()
    }
}

/// Expand module-level inline procedural macro.
#[tracing::instrument(level = "trace", skip_all)]
pub fn expand_module_level_inline_macro<'db>(
    host: &ProcMacroHostPlugin,
    db: &'db dyn Database,
    inline_macro: &ast::ItemInlineMacro<'db>,
    _stream_metadata: &TokenStreamMetadata,
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

    let result = host
        .instance(found.package_id)
        .try_v2()
        .expect("procedural macro using v1 api used in a context expecting v2 api")
        .generate_code(
            found.expansion.expansion_name.clone(),
            adapted_call_site.clone(),
            TokenStream::empty(),
            adapted_token_stream,
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

    let aux_data = result.aux_data.map(|aux_data| {
        DynGeneratedFileAuxData::new(EmittedAuxData::new(ProcMacroAuxData::new(
            aux_data.into(),
            found.clone(),
        )))
    });
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
                found.expansion.cairo_name
            )),
            is_unhygienic: false,
        }),
        diagnostics,
        remove_original_item: true,
    })
}
