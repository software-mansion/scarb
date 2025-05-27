//! Adapter used to re-calculate token span locations when expanding attributes to account for
//! the expandable attribute that is removed from the expansion input.
//!
//! The proc macro attributes are expanded in the order they appear in the code, from top to bottom.
//! If an attribute representing an expandable procedural macro is found,
//! it's removed from the attributes list for the expansion input.
//! This poses a problem, as procedural macro implementation must assume that the `TokenStream`
//! provided as an input is consecutive. This limitation comes from how the token stream parser
//! works, as it only sets the initial offset, and generates the rest of the spans as it would
//! when parsing a source file. Obviously, when we remove the attribute from the `TokenStream`
//! built, it's no longer consecutive.
//!
//! See [`crate::compiler::plugin::proc_macro::v2::ProcMacroHostPlugin::parse_attribute`] for more context.
//!
//! We mitigate this problem, by following logic:
//! Spans in the expansion input and code mappings generated from the expansion output are moved
//! around, as if the expandable attribute was the first attribute in the attributes list.
//! *Note that no code is actually rewritten - only the corresponding token spans are modified.*
//! Input `TokenStream` is built by the following rules:
//! - All spans of tokens before the expandable attribute are moved towards the end of the file
//!   by the expandable attribute length.
//! - Tokens representing the expandable attribute are skipped.
//! - All tokens after the expandable attribute are added as is. We can do it this way, as sum
//!   of the lengths of tokens before the expandable attribute plus the length of the
//!   expandable attribute is always the same, regardless of their order.
//! - We save the start offset of the removed attribute alongside the expansion arguments, to be
//!   used later when generating code mappings.
//! - As call site, we pass the span between beginning of the file and attribute length - as if
//!   the expandable attribute was the first attribute in the attributes list.
//!   Code mappings for the `TokenStream` are generated according to following rules:
//! - We iterate over the resulting `TokenStream`.
//! - Spans that end after the end offset of the removed attribute (i.e. start offset + length),
//!   are left as is. Those spans have not been moved before neither.
//! - Spans that start after the expandable attribute length, but before the end offset of the
//!   expandable attribute, are moved towards the beginning of the file by the expandable
//!   attribute length.
//! - Spans that start before the expandable attribute length, are moved towards the end of
//!   the file by the start offset of the expandable attribute.
//! - This includes moving the call site.
//!
//! The code mapping modifications happen after the macro expansion, in `expand_attribute` method.
//! This can be visualized as:
//! Original file:
//! |(0) some attributes |(start offset) expandable attribute |(end offset) other attributes and body|
//! Expansion input:
//! -> some attributes += attribute length
//! -> expandable attribute -= start offset
//! |(0) expandable attribute |(attribute length) some attributes |(end offset) other attributes and body|
//! Expansion output:
//! -> some attributes -= attribute length
//! -> expandable attribute += start offset
//! |(0) some attributes |(start offset) expandable attribute |(end offset) other attributes and body|
//! Remember, we only move the spans, not the actual code!

use crate::compiler::plugin::proc_macro::v2::host::attribute::{
    AttrExpansionFound, ExpandableAttrLocation,
};
use cairo_lang_filesystem::ids::{CodeMapping, CodeOrigin};
use cairo_lang_filesystem::span::TextSpan as CairoTextSpan;
use cairo_lang_filesystem::span::TextWidth;
use cairo_lang_macro::{
    Diagnostic, TextOffset, TextSpan, TokenStream, TokenStreamMetadata, TokenTree,
};
use std::fmt::Display;

/// Move spans in the `TokenStream` for macro expansion input.
pub fn move_spans(
    input: &AttrExpansionFound,
    token_stream: TokenStream,
) -> TokenStreamAdaptedLocation {
    let attribute_location = match &input {
        AttrExpansionFound::Some(args) | AttrExpansionFound::Last(args) => &args.attribute_location,
        AttrExpansionFound::None => return TokenStreamAdaptedLocation(token_stream),
    };
    let (start, len) = (
        attribute_location.token_offset,
        attribute_location.token_length.as_u32(),
    );
    let token_stream = TokenStream::new(
        token_stream
            .into_iter()
            .map(|tree| match tree {
                TokenTree::Ident(mut token) => {
                    if token.span.start < start {
                        token.span.start += len;
                        token.span.end += len;
                    }
                    TokenTree::Ident(token)
                }
            })
            .collect(),
    );
    TokenStreamAdaptedLocation(token_stream)
}

/// Move code mappings to account for the removed expandable attribute for the expansion output.
pub fn move_mappings_by_expanded_attr(
    code_mappings: Vec<CodeMapping>,
    attribute_span: &ExpandableAttrLocation,
) -> Vec<CodeMapping> {
    let attr_offset = attribute_span.token_offset;
    let attr_length = attribute_span.token_length;
    code_mappings
        .into_iter()
        .map(|code_mapping| {
            let origin = match code_mapping.origin {
                CodeOrigin::Span(span) => {
                    let span = if span.start < attr_length.as_offset() {
                        CairoTextSpan {
                            start: span
                                .start
                                .add_width(TextWidth::new_for_testing(attr_offset)),
                            end: span.end.add_width(TextWidth::new_for_testing(attr_offset)),
                        }
                    } else if span.start.as_u32() < attr_length.as_u32() + attr_offset {
                        CairoTextSpan {
                            start: span.start.sub_width(attr_length),
                            end: span.end.sub_width(attr_length),
                        }
                    } else {
                        span
                    };
                    CodeOrigin::Span(span)
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

pub fn move_diagnostics_span_by_expanded_attr(
    diagnostics: Vec<Diagnostic>,
    attribute_span: &ExpandableAttrLocation,
) -> Vec<Diagnostic> {
    diagnostics
        .into_iter()
        .map(|mut diagnostic| {
            if let Some(span) = diagnostic.span.as_mut() {
                if span.start < attribute_span.token_length.as_u32() {
                    span.start += attribute_span.token_offset;
                    span.end += attribute_span.token_offset;
                } else if span.start
                    < attribute_span.token_length.as_u32() + attribute_span.token_offset
                {
                    span.start -= attribute_span.token_length.as_u32();
                    span.end -= attribute_span.token_length.as_u32();
                }
            }
            diagnostic
        })
        .collect()
}

#[derive(Clone)]
pub struct TokenStreamAdaptedLocation(TokenStream);

impl TokenStreamAdaptedLocation {
    pub fn with_metadata(self, metadata: TokenStreamMetadata) -> Self {
        Self(self.0.with_metadata(metadata))
    }
}

impl From<TokenStreamAdaptedLocation> for TokenStream {
    fn from(value: TokenStreamAdaptedLocation) -> Self {
        value.0
    }
}

impl Display for TokenStreamAdaptedLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn adapt_call_site_span(span: TextSpan, token_offset: TextOffset) -> TextSpan {
    TextSpan {
        start: span.start - token_offset,
        end: span.end - token_offset,
    }
}
