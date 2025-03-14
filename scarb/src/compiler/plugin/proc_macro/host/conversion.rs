use crate::compiler::plugin::proc_macro::ExpansionKind;
use cairo_lang_defs::plugin::PluginDiagnostic;
use cairo_lang_filesystem::span::TextWidth;
use cairo_lang_macro::{Diagnostic, Severity, TextSpan};
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::ids::SyntaxStablePtrId;
use cairo_lang_syntax::node::stable_ptr::SyntaxStablePtr;
use cairo_lang_syntax::node::{TypedStablePtr, TypedSyntaxNode};
use cairo_lang_utils::LookupIntern;
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
            stable_ptr: node.stable_ptr().untyped(),
            span: node.text_span(db),
        }
    }
}

pub fn into_cairo_diagnostics(
    db: &dyn SyntaxGroup,
    diagnostics: Vec<Diagnostic>,
    stable_ptr: SyntaxStablePtrId,
    origin_names: Option<&str>,
    macro_type: Option<ExpansionKind>,
) -> Vec<PluginDiagnostic> {
    let root_stable_ptr = get_root_ptr(db, stable_ptr);
    let root_syntax_node = root_stable_ptr.lookup(db);
    stable_ptr.file_id(db);

    diagnostics
        .into_iter()
        .map(|diag| {
            let severity = match diag.severity {
                Severity::Error => cairo_lang_diagnostics::Severity::Error,
                Severity::Warning => cairo_lang_diagnostics::Severity::Warning,
            };

            if let Some(span) = diag.span {
                // Diagnostic with a location specified
                let (start_ptr, end_ptr) = span_to_stable_ptrs(db, &span, &root_syntax_node);
                let note = if let (Some(name), Some(kind)) = (origin_names, macro_type.clone()) {
                    Some(diagnostic_note(name, &kind))
                } else {
                    None
                };

                PluginDiagnostic {
                    stable_ptr: start_ptr,
                    end_ptr: Some(end_ptr),
                    message: diag.message,
                    severity,
                    note: note.clone(),
                }
            } else {
                PluginDiagnostic {
                    stable_ptr,
                    end_ptr: None,
                    message: diag.message,
                    severity,
                    note: None,
                }
            }
        })
        .collect_vec()
}

fn get_root_ptr(db: &dyn SyntaxGroup, stable_ptr: SyntaxStablePtrId) -> SyntaxStablePtrId {
    let mut current_ptr = stable_ptr;

    while let SyntaxStablePtr::Child {
        parent: parent_ptr,
        kind: _,
        key_fields: _,
        index: _,
    } = current_ptr.lookup_intern(db)
    {
        current_ptr = parent_ptr;
    }
    current_ptr
}

fn diagnostic_note(macro_name: &str, macro_type: &ExpansionKind) -> String {
    let macro_type_str = match macro_type {
        ExpansionKind::Attr => "attribute macro",
        ExpansionKind::Derive => "derive macro(s)",
        ExpansionKind::Inline => "inline macro",
        ExpansionKind::Executable => "executable macro",
    };
    format!(
        "this diagnostic originates in the {}: `{}`",
        macro_type_str, macro_name
    )
}

fn span_to_stable_ptrs(
    db: &dyn SyntaxGroup,
    span: &TextSpan,
    root_syntax_node: &cairo_lang_syntax::node::SyntaxNode,
) -> (SyntaxStablePtrId, SyntaxStablePtrId) {
    let start_offset = cairo_lang_filesystem::span::TextOffset::default()
        .add_width(TextWidth::new_for_testing(span.start));
    let end_offset = cairo_lang_filesystem::span::TextOffset::default()
        .add_width(TextWidth::new_for_testing(span.end));

    let start_node = root_syntax_node.lookup_offset(db, start_offset);
    let end_node = root_syntax_node.lookup_offset(db, end_offset);

    (start_node.stable_ptr(), end_node.stable_ptr())
}
