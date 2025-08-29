use cairo_lang_defs::plugin::PluginDiagnostic;
use cairo_lang_filesystem::span::{TextOffset, TextWidth};
use cairo_lang_macro::{Diagnostic, Severity, TextSpan};
use cairo_lang_syntax::node::ids::SyntaxStablePtrId;
use cairo_lang_syntax::node::stable_ptr::SyntaxStablePtr;
use cairo_lang_syntax::node::{SyntaxNode, TypedStablePtr, TypedSyntaxNode};
use itertools::Itertools;
use salsa::Database;

pub trait SpanSource<'db> {
    fn text_span(&self, db: &'db dyn Database) -> TextSpan;
}

impl<'db, T: TypedSyntaxNode<'db>> SpanSource<'db> for T {
    fn text_span(&self, db: &'db dyn Database) -> TextSpan {
        let node = self.as_syntax_node();
        let span = node.span_without_trivia(db);
        TextSpan::new(span.start.as_u32(), span.end.as_u32())
    }
}

pub struct CallSiteLocation<'db> {
    pub stable_ptr: SyntaxStablePtrId<'db>,
    pub span: TextSpan,
}

impl<'db> CallSiteLocation<'db> {
    pub fn new<T: TypedSyntaxNode<'db>>(node: &T, db: &'db dyn Database) -> Self {
        Self {
            stable_ptr: node.stable_ptr(db).untyped(),
            span: node.text_span(db),
        }
    }
}

pub fn into_cairo_diagnostics<'db>(
    db: &'db dyn Database,
    diagnostics: Vec<Diagnostic>,
    call_site_stable_ptr: SyntaxStablePtrId<'db>,
) -> Vec<PluginDiagnostic<'db>> {
    let root_stable_ptr = get_root_ptr(db, call_site_stable_ptr);
    let root_syntax_node = root_stable_ptr.lookup(db);
    diagnostics
        .into_iter()
        .map(|diag| {
            // Resolve the best possible diagnostic location.
            // If the diagnostic span is provided, find the encompassing node and compute the span relative to that node.
            // Fall back to the call-site stable pointer, if diagnostic span is not provided or if the encompassing node cannot be found.
            let (node_stable_ptr, inner_span) = if let Some(span) = diag.span() {
                if let Some(node) = find_encompassing_node(&root_syntax_node, db, &span) {
                    let inner_span = compute_relative_span(&node, db, &span);
                    (node.stable_ptr(db), Some(inner_span))
                } else {
                    (call_site_stable_ptr, None)
                }
            } else {
                (call_site_stable_ptr, None)
            };
            PluginDiagnostic {
                stable_ptr: node_stable_ptr,
                message: diag.message().to_string(),
                severity: match diag.severity() {
                    Severity::Error => cairo_lang_diagnostics::Severity::Error,
                    Severity::Warning => cairo_lang_diagnostics::Severity::Warning,
                },
                inner_span,
            }
        })
        .collect_vec()
}

fn get_root_ptr<'db>(
    db: &'db dyn Database,
    stable_ptr: SyntaxStablePtrId<'db>,
) -> SyntaxStablePtrId<'db> {
    let mut current_ptr = stable_ptr;

    while let SyntaxStablePtr::Child {
        parent: parent_ptr,
        kind: _,
        key_fields: _,
        index: _,
    } = current_ptr.long(db)
    {
        current_ptr = *parent_ptr;
    }
    current_ptr
}

/// Finds the most specific node that fully encompasses the given text span.
/// Returns `None` if unable to find such node.
pub fn find_encompassing_node<'db>(
    root_syntax_node: &SyntaxNode<'db>,
    db: &'db dyn Database,
    span: &TextSpan,
) -> Option<SyntaxNode<'db>> {
    let start_offset = TextOffset::default().add_width(TextWidth::new_for_testing(span.start));
    let end_offset = TextOffset::default().add_width(TextWidth::new_for_testing(span.end));

    let mut current_node = root_syntax_node.lookup_offset(db, start_offset);
    while current_node.span(db).end < end_offset {
        if let Some(parent) = current_node.parent(db) {
            current_node = parent;
        } else {
            return None;
        }
    }
    Some(current_node)
}

/// Computes a span relative to `node` from an `absolute_span`.
fn compute_relative_span<'db>(
    node: &SyntaxNode<'db>,
    db: &'db dyn Database,
    absolute_span: &TextSpan,
) -> (TextWidth, TextWidth) {
    let offset = node.offset(db).as_u32();
    let relative_start = absolute_span.start.saturating_sub(offset);
    let relative_end = absolute_span.end.saturating_sub(offset);
    (
        TextWidth::new_for_testing(relative_start),
        TextWidth::new_for_testing(relative_end - relative_start),
    )
}
