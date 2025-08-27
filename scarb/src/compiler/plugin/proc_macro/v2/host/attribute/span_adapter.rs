//! Adapter used to re-calculate token span locations when expanding attributes to account for
//! the expandable attribute that is removed from the expansion input.
//!
//! The proc macro attributes are expanded in the order they appear in the code, from top to bottom.
//! If an attribute representing an expandable procedural macro is found,
//! it's removed from the attributes list for the expansion input.
//! This poses a problem, as procedural macro implementation must assume that the `TokenStream`
//! provided as an input is consecutive, i.e. it represents some part of a Cairo source code,
//! without any gaps. This limitation comes from how the token stream parser works, as it only sets
//! the initial offset, and generates the rest of the spans as it would when parsing a source file.
//! Obviously, when we remove the attribute from the `TokenStream` built, it's no longer consecutive.
//!
//! See [`crate::compiler::plugin::proc_macro::v2::ProcMacroHostPlugin::parse_attribute`] for more context.
//!
//! We mitigate this problem by following logic:
//! Spans in the expansion input and code mappings generated from the expansion output are moved
//! around, as if the expandable attribute was the last token in the built token stream.
//! *Note that no code is actually rewritten - only the corresponding token spans are modified.*
//! Input `TokenStream` is built by the following rules:
//! - All spans of tokens before the expandable attribute are moved towards the beginning of the
//!   file, so that the first token start offset is 0.
//! - Tokens representing the expandable attribute are skipped.
//! - All tokens after the expandable attribute are moved towards the beginning of the file, so that
//!   they start consecutively after the attributes described above.
//! - We save the start offset of the removed attribute alongside the expansion arguments, to be
//!   used later when generating code mappings.
//! - As call site, we pass the span after the length of the not removed code and the expandable
//!   attribute length - as if the expandable attribute was the last token we pass to the token stream.
//!
//! Code mappings for the `TokenStream` are generated so that we revert the move described above.
//! The code mapping modifications happen after the macro expansion, in `expand_attribute` method.

//! This can be visualized as:
//! Original file:
//! |(first token offset) some attributes |(start offset) expandable attribute |(end offset) other attributes and body|
//! Expansion input:
//! |(0) some attributes |(start offset - first token offset) other attributes and body | (end offset - first token offset - expandable attribute width) expandable attribute
//! Expansion output:
//! |(first token offset) some attributes |(start offset) expandable attribute |(end offset) other attributes and body|
//! Remember, we only move the spans, not the actual code!

use crate::compiler::plugin::proc_macro::v2::host::attribute::AttrExpansionFound;
use crate::compiler::plugin::proc_macro::v2::host::conversion::SpanSource;
use cairo_lang_filesystem::ids::{CodeMapping, CodeOrigin};
use cairo_lang_filesystem::span::TextSpan as CairoTextSpan;
use cairo_lang_filesystem::span::TextWidth;
use cairo_lang_macro::{
    Diagnostic, TextOffset, TextSpan, TokenStream, TokenStreamMetadata, TokenTree,
};
use cairo_lang_syntax::node::TypedSyntaxNode;
use salsa::Database;
use std::fmt::Display;

/// [`TokenStream`] with token spans adapted for expansion input.
#[derive(Clone, Debug)]
pub struct AdaptedTokenStream(TokenStream);

impl AdaptedTokenStream {
    pub fn with_metadata(self, metadata: TokenStreamMetadata) -> Self {
        Self(self.0.with_metadata(metadata))
    }
}

impl From<AdaptedTokenStream> for TokenStream {
    fn from(value: AdaptedTokenStream) -> Self {
        value.0
    }
}

impl Display for AdaptedTokenStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub struct AdaptedCodeMapping(CodeMapping);

impl From<AdaptedCodeMapping> for CodeMapping {
    fn from(value: AdaptedCodeMapping) -> Self {
        value.0
    }
}

#[derive(Debug)]
pub struct AdaptedDiagnostic(Diagnostic);

impl From<AdaptedDiagnostic> for Diagnostic {
    fn from(value: AdaptedDiagnostic) -> Self {
        value.0
    }
}

#[derive(Debug)]
pub struct AdaptedTextSpan(TextSpan);

impl From<AdaptedTextSpan> for TextSpan {
    fn from(value: AdaptedTextSpan) -> Self {
        value.0
    }
}

/// This struct represents the location of the attribute expansion call site (expandable attribute).
///
/// It contains both the original location of the attribute in the source code file and the adapted
/// location, i.e. as if the attribute was the first attribute in the attributes list of that token
/// stream.
pub struct ExpandableAttrLocation {
    span_with_trivia: TextSpan,
    span_without_trivia: TextSpan,
    // This refers to the whole TokenStream we build.
    whole_item_span: TextSpan,
}

impl ExpandableAttrLocation {
    pub fn new<'db, T: TypedSyntaxNode<'db>>(
        node: &T,
        item_span: CairoTextSpan,
        db: &'db dyn Database,
    ) -> Self {
        let span_without_trivia = node.text_span(db);
        let span_with_trivia = node.as_syntax_node().span(db);
        Self {
            span_with_trivia: TextSpan {
                start: span_with_trivia.start.as_u32(),
                end: span_with_trivia.end.as_u32(),
            },
            span_without_trivia,
            whole_item_span: TextSpan {
                start: item_span.start.as_u32(),
                end: item_span.end.as_u32(),
            },
        }
    }

    fn start_offset_with_trivia(&self) -> TextOffset {
        self.span_with_trivia.start
    }

    fn end_offset_with_trivia(&self) -> TextOffset {
        self.span_with_trivia.end
    }

    fn width_with_trivia(&self) -> u32 {
        self.span_with_trivia.end - self.span_with_trivia.start
    }

    fn width_without_trivia(&self) -> u32 {
        self.span_without_trivia.end - self.span_without_trivia.start
    }

    pub fn adapted_call_site(&self) -> AdaptedTextSpan {
        let start =
            self.whole_item_span.end - self.width_without_trivia() - self.whole_item_span.start;
        AdaptedTextSpan(TextSpan {
            start,
            end: start + self.width_without_trivia(),
        })
    }

    /// Move spans in the `TokenStream` for macro expansion input.
    pub fn adapt_token_stream(&self, token_stream: TokenStream) -> AdaptedTokenStream {
        let attr_start = self.start_offset_with_trivia();
        let attr_end = self.end_offset_with_trivia();
        let attr_width = self.width_with_trivia();
        let whole_item_width = self.whole_item_span.end - self.whole_item_span.start;
        let token_stream = TokenStream::new(
            token_stream
                .into_iter()
                .map(|tree| match tree {
                    TokenTree::Ident(mut token) => {
                        if token.span.start < attr_start {
                            // Some attributes before the expandable attribute.
                            token.span.start -= self.whole_item_span.start;
                            token.span.end -= self.whole_item_span.start;
                        } else if token.span.end < attr_end {
                            // The expandable attribute itself.
                            token.span.start += whole_item_width - attr_width - attr_start;
                            token.span.end += whole_item_width - attr_width - attr_start;
                        } else {
                            // The code after the expandable attribute.
                            token.span.start -= attr_width + self.whole_item_span.start;
                            token.span.end -= attr_width + self.whole_item_span.start;
                        }
                        TokenTree::Ident(token)
                    }
                })
                .collect(),
        );
        AdaptedTokenStream(token_stream)
    }

    /// Move code mappings to account for the removed expandable attribute for the expansion output.
    pub fn adapt_code_mappings(&self, code_mappings: Vec<CodeMapping>) -> Vec<AdaptedCodeMapping> {
        let attr_start = self.start_offset_with_trivia();
        let attr_width = self.width_with_trivia();
        let whole_item_width = self.whole_item_span.end - self.whole_item_span.start;
        let move_callsite = |span: CairoTextSpan| CairoTextSpan {
            start: span
                .start
                .add_width(TextWidth::new_for_testing(
                    self.width_without_trivia() + self.span_without_trivia.start,
                ))
                .sub_width(TextWidth::new_for_testing(whole_item_width)),
            end: span
                .end
                .add_width(TextWidth::new_for_testing(
                    self.width_without_trivia() + self.span_without_trivia.start,
                ))
                .sub_width(TextWidth::new_for_testing(whole_item_width)),
        };
        code_mappings
            .into_iter()
            .map(|code_mapping| {
                let origin = match code_mapping.origin {
                    CodeOrigin::Span(span) => {
                        let span = if span.start.as_u32() < attr_start - self.whole_item_span.start
                        {
                            // Some attributes before the expandable attribute.
                            CairoTextSpan {
                                start: span.start.add_width(TextWidth::new_for_testing(
                                    self.whole_item_span.start,
                                )),
                                end: span.end.add_width(TextWidth::new_for_testing(
                                    self.whole_item_span.start,
                                )),
                            }
                        } else if span.start.as_u32() < whole_item_width - attr_width {
                            // The code after the expandable attribute.
                            CairoTextSpan {
                                start: span.start.add_width(TextWidth::new_for_testing(
                                    attr_width + self.whole_item_span.start,
                                )),
                                end: span.end.add_width(TextWidth::new_for_testing(
                                    attr_width + self.whole_item_span.start,
                                )),
                            }
                        } else {
                            // The expandable attribute itself.
                            move_callsite(span)
                        };
                        CodeOrigin::Span(span)
                    }
                    CodeOrigin::CallSite(span) => CodeOrigin::CallSite(move_callsite(span)),
                    origin => origin,
                };
                CodeMapping {
                    span: code_mapping.span,
                    origin,
                }
            })
            .map(AdaptedCodeMapping)
            .collect()
    }

    /// Move spans in diagnostics to account for the removed expandable attribute for the expansion output.
    pub fn adapt_diagnostics(&self, diagnostics: Vec<Diagnostic>) -> Vec<AdaptedDiagnostic> {
        let attr_start = self.start_offset_with_trivia();
        let attr_width = self.width_with_trivia();
        let whole_item_width = self.whole_item_span.end - self.whole_item_span.start;
        diagnostics
            .into_iter()
            .map(|diagnostic| {
                if let Some(mut span) = diagnostic.span() {
                    if span.start < attr_start - self.whole_item_span.start {
                        // Some attributes before the expandable attribute.
                        span.start += self.whole_item_span.start;
                        span.end += self.whole_item_span.start;
                    } else if span.start < whole_item_width - attr_width {
                        // The code after the expandable attribute.
                        span.start += attr_width + self.whole_item_span.start;
                        span.end += attr_width + self.whole_item_span.start;
                    } else {
                        // The expandable attribute itself.
                        span.start += self.width_without_trivia() + self.span_without_trivia.start
                            - whole_item_width;
                        span.end += self.width_without_trivia() + self.span_without_trivia.start
                            - whole_item_width;
                    }
                    Diagnostic::spanned(span, diagnostic.severity(), diagnostic.message())
                } else {
                    diagnostic
                }
            })
            .map(AdaptedDiagnostic)
            .collect()
    }
}

impl<'db> AttrExpansionFound<'db> {
    /// Move spans in the `TokenStream` for macro expansion input.
    pub fn adapt_token_stream(&self, token_stream: TokenStream) -> AdaptedTokenStream {
        match self {
            AttrExpansionFound::Some(args) | AttrExpansionFound::Last(args) => {
                args.attribute_location.adapt_token_stream(token_stream)
            }
            AttrExpansionFound::None => AdaptedTokenStream(token_stream),
        }
    }
}
