use cairo_lang_defs::plugin::PluginDiagnostic;
use cairo_lang_filesystem::span::{TextOffset, TextWidth};
use cairo_lang_macro::{Diagnostic, Severity, TextSpan};
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::ids::SyntaxStablePtrId;
use cairo_lang_syntax::node::stable_ptr::SyntaxStablePtr;
use cairo_lang_syntax::node::{SyntaxNode, TypedStablePtr, TypedSyntaxNode};
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
            stable_ptr: node.stable_ptr(db).untyped(),
            span: node.text_span(db),
        }
    }
}

pub fn into_cairo_diagnostics(
    db: &dyn SyntaxGroup,
    diagnostics: Vec<Diagnostic>,
    call_site_stable_ptr: SyntaxStablePtrId,
) -> Vec<PluginDiagnostic> {
    let root_stable_ptr = get_root_ptr(db, call_site_stable_ptr);
    let root_syntax_node = root_stable_ptr.lookup(db);
    diagnostics
        .into_iter()
        .map(|diag| {
            let (node_stable_ptr, relative_span) = match diag.span {
                Some(span) => {
                    if let Some(node) = find_encompassing_node(&root_syntax_node, db, &span) {
                        let offset = node.offset(db).as_u32();
                        let relative_span = cairo_lang_filesystem::span::TextSpan {
                            start: TextOffset::default()
                                .add_width(TextWidth::new_for_testing(span.start))
                                .sub_width(TextWidth::new_for_testing(offset)),
                            end: TextOffset::default()
                                .add_width(TextWidth::new_for_testing(span.end))
                                .sub_width(TextWidth::new_for_testing(offset)),
                        };
                        (node.stable_ptr(db), Some(relative_span))
                    } else {
                        (call_site_stable_ptr, None)
                    }
                },
                None => (call_site_stable_ptr, None),
            };

            PluginDiagnostic {
                stable_ptr: node_stable_ptr,
                relative_span,
                message: diag.message,
                severity: match diag.severity {
                    Severity::Error => cairo_lang_diagnostics::Severity::Error,
                    Severity::Warning => cairo_lang_diagnostics::Severity::Warning,
                },
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

/// Finds the most specific node that fully encompasses the given text span.
/// Returns `None` if unable to find such node.
pub fn find_encompassing_node(
    root_syntax_node: &SyntaxNode,
    db: &dyn SyntaxGroup,
    span: &TextSpan,
) -> Option<SyntaxNode> {
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
