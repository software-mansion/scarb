use crate::compiler::plugin::proc_macro::v2::host::span_utils::move_spans_by_offset;
use cairo_lang_filesystem::ids::{CodeMapping, CodeOrigin};
use cairo_lang_filesystem::span::{TextSpan as CairoTextSpan, TextWidth};
use cairo_lang_macro::{Diagnostic, TextOffset, TextSpan, Token, TokenStream, TokenTree};

pub struct InlineAdapter {
    initial_offset: TextOffset,
    item_span: CairoTextSpan,
    call_site_span: TextSpan,
}

impl InlineAdapter {
    /// Move spans in the `TokenStream` for macro expansion input.
    pub fn adapt_token_stream(
        token_stream: TokenStream,
        item_span: CairoTextSpan,
        call_site_span: TextSpan,
    ) -> (Self, TokenStream) {
        let this = if let Some(TokenTree::Ident(Token { span, .. })) = token_stream.tokens.first() {
            Self {
                initial_offset: span.start,
                item_span,
                call_site_span,
            }
        } else {
            Self {
                initial_offset: 0,
                item_span,
                call_site_span,
            }
        };
        let token_stream = move_spans_by_offset(token_stream, this.initial_offset);
        (this, token_stream)
    }

    pub fn adapted_call_site(&self) -> TextSpan {
        let call_site_width = self.call_site_span.end - self.call_site_span.start;
        TextSpan {
            start: self.item_width(),
            end: self.item_width() + call_site_width,
        }
    }

    /// Move code mappings to account for the relativization.
    pub fn adapt_code_mappings(&self, code_mappings: Vec<CodeMapping>) -> Vec<CodeMapping> {
        let move_callsite = |mut span: CairoTextSpan| {
            span.start = span
                .start
                .add_width(TextWidth::new_for_testing(self.call_site_span.start))
                .sub_width(TextWidth::new_for_testing(self.item_width()));
            span.end = span
                .end
                .add_width(TextWidth::new_for_testing(self.call_site_span.start))
                .sub_width(TextWidth::new_for_testing(self.item_width()));
            span
        };
        code_mappings
            .into_iter()
            .map(|code_mapping| {
                let origin = match code_mapping.origin {
                    CodeOrigin::Span(mut span) => {
                        if span.start.as_u32() >= self.item_width() {
                            CodeOrigin::Span(move_callsite(span))
                        } else {
                            span.start = span
                                .start
                                .add_width(TextWidth::new_for_testing(self.initial_offset));
                            span.end = span
                                .end
                                .add_width(TextWidth::new_for_testing(self.initial_offset));
                            CodeOrigin::Span(span)
                        }
                    }
                    CodeOrigin::CallSite(span) => CodeOrigin::CallSite(move_callsite(span)),
                    origin => origin,
                };
                CodeMapping {
                    span: code_mapping.span,
                    origin,
                }
            })
            .collect()
    }

    /// Move code mappings to account for the relativization.
    pub fn adapt_diagnostics(&self, diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
        diagnostics
            .into_iter()
            .map(|diagnostic| {
                if let Some(mut span) = diagnostic.span() {
                    if span.start >= self.item_width() {
                        span.start += self.call_site_span.start - self.item_width();
                        span.end += self.call_site_span.start - self.item_width();
                    } else {
                        span.start += self.initial_offset;
                        span.end += self.initial_offset;
                    }
                    Diagnostic::spanned(span, diagnostic.severity(), diagnostic.message())
                } else {
                    diagnostic
                }
            })
            .collect()
    }

    fn item_width(&self) -> u32 {
        self.item_span.end.as_u32() - self.item_span.start.as_u32()
    }
}
