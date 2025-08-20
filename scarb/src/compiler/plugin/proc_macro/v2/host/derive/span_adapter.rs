use cairo_lang_filesystem::ids::{CodeMapping, CodeOrigin};
use cairo_lang_filesystem::span::TextWidth;
use cairo_lang_macro::{Diagnostic, TextOffset, TextSpan, Token, TokenStream, TokenTree};

pub struct DeriveAdapter {
    initial_offset: TextOffset,
}

impl DeriveAdapter {
    /// Move spans in the `TokenStream` for macro expansion input.
    pub fn adapt_token_stream(token_stream: TokenStream) -> (Self, TokenStream) {
        let this = if let Some(TokenTree::Ident(Token { span, .. })) = token_stream.tokens.first() {
            Self {
                initial_offset: span.start,
            }
        } else {
            Self { initial_offset: 0 }
        };
        let token_stream = TokenStream::new(
            token_stream
                .into_iter()
                .map(|tree| match tree {
                    TokenTree::Ident(mut token) => {
                        token.span.start -= this.initial_offset;
                        token.span.end -= this.initial_offset;
                        TokenTree::Ident(token)
                    }
                })
                .collect(),
        );
        (this, token_stream)
    }

    pub fn adapted_call_site(&self, call_site: &TextSpan) -> TextSpan {
        TextSpan {
            start: call_site.start - self.initial_offset,
            end: call_site.end - self.initial_offset,
        }
    }

    /// Move code mappings to account for the relativization.
    pub fn adapt_code_mappings(&self, code_mappings: Vec<CodeMapping>) -> Vec<CodeMapping> {
        code_mappings
            .into_iter()
            .map(|code_mapping| {
                let origin = match code_mapping.origin {
                    CodeOrigin::Span(mut span) => {
                        span.start = span
                            .start
                            .add_width(TextWidth::new_for_testing(self.initial_offset));
                        span.end = span
                            .end
                            .add_width(TextWidth::new_for_testing(self.initial_offset));
                        CodeOrigin::Span(span)
                    }
                    CodeOrigin::CallSite(mut span) => {
                        span.start = span
                            .start
                            .add_width(TextWidth::new_for_testing(self.initial_offset));
                        span.end = span
                            .end
                            .add_width(TextWidth::new_for_testing(self.initial_offset));
                        CodeOrigin::CallSite(span)
                    }
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
                    span.start += self.initial_offset;
                    span.end += self.initial_offset;
                    Diagnostic::spanned(span, diagnostic.severity(), diagnostic.message())
                } else {
                    diagnostic
                }
            })
            .collect()
    }
}
