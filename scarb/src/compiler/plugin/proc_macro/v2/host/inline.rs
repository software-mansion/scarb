use crate::compiler::plugin::proc_macro::ProcMacroInstance;
use crate::compiler::plugin::proc_macro::expansion::Expansion;
use crate::compiler::plugin::proc_macro::v2::host::aux_data::{EmittedAuxData, ProcMacroAuxData};
use crate::compiler::plugin::proc_macro::v2::host::conversion::{
    CallSiteLocation, into_cairo_diagnostics,
};
use crate::compiler::plugin::proc_macro::v2::host::generate_code_mappings;
use crate::compiler::plugin::proc_macro::v2::{ProcMacroId, TokenStreamBuilder};
use cairo_lang_defs::plugin::{
    DynGeneratedFileAuxData, InlineMacroExprPlugin, InlinePluginResult, MacroPluginMetadata,
    PluginGeneratedFile,
};
use cairo_lang_macro::{AllocationContext, TokenStream};
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::{TypedSyntaxNode, ast};
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
    fn generate_code(
        &self,
        db: &dyn SyntaxGroup,
        syntax: &ast::ExprInlineMacro,
        _metadata: &MacroPluginMetadata<'_>,
    ) -> InlinePluginResult {
        let call_site = CallSiteLocation::new(syntax, db);
        let ctx = AllocationContext::default();
        let arguments = syntax.arguments(db);
        let mut token_stream_builder = TokenStreamBuilder::new(db);
        token_stream_builder.add_node(arguments.as_syntax_node());
        let token_stream = token_stream_builder.build(&ctx);
        let result = self
            .instance()
            .try_v2()
            .expect("procedural macro using v1 api used in a context expecting v2 api")
            .generate_code(
                self.expansion.expansion_name.clone(),
                call_site.span,
                TokenStream::empty(),
                token_stream,
            );
        // Handle diagnostics.
        let diagnostics = into_cairo_diagnostics(db, result.diagnostics, call_site.stable_ptr);
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
            let code_mappings = generate_code_mappings(&token_stream);
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
