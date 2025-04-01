use cairo_lang_defs::plugin::PluginDiagnostic;
use cairo_lang_filesystem::span::TextWidth;
use cairo_lang_macro::{Diagnostic, Severity, TextSpan};
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::ids::SyntaxStablePtrId;
use cairo_lang_syntax::node::{TypedStablePtr, TypedSyntaxNode};
use itertools::Itertools;

pub trait SpanSource {
    fn text_span(&self, db: &dyn SyntaxGroup) -> TextSpan;
}

impl<T: TypedSyntaxNode> SpanSource for T {
    fn text_span(&self, db: &dyn SyntaxGroup) -> TextSpan {
        let node = self.as_syntax_node();
        let span = node.span(db);
        TextSpan::new(span.start.as_u32(), span.end.as_u32())
    }
}

pub struct CallSiteLocation {
    pub stable_ptr: SyntaxStablePtrId,
    pub span: TextSpan,
}

impl CallSiteLocation {
    pub fn new<T: TypedSyntaxNode>(node: &T, db: &dyn SyntaxGroup) -> Self {
        Self {
            stable_ptr: node.stable_ptr(db).untyped(),
            span: node.text_span(db),
        }
    }
}

pub fn into_cairo_diagnostics(
    diagnostics: Vec<Diagnostic>,
    stable_ptr: SyntaxStablePtrId,
) -> Vec<PluginDiagnostic> {
    diagnostics
        .into_iter()
        .map(|diag| PluginDiagnostic {
            stable_ptr,
            relative_span: diag.span.map(into_cairo_span),
            message: diag.message,
            severity: match diag.severity {
                Severity::Error => cairo_lang_diagnostics::Severity::Error,
                Severity::Warning => cairo_lang_diagnostics::Severity::Warning,
            },
            relative_span: Default::default(),
        })
        .collect_vec()
}

pub fn into_cairo_span(span: TextSpan) -> cairo_lang_filesystem::span::TextSpan {
    cairo_lang_filesystem::span::TextSpan {
        start: cairo_lang_filesystem::span::TextOffset::default()
            .add_width(TextWidth::new_for_testing(span.start)),
        end: cairo_lang_filesystem::span::TextOffset::default()
            .add_width(TextWidth::new_for_testing(span.end)),
    }
}
