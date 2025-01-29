use std::iter::Peekable;

use proc_macro2::{Delimiter, Ident, Span, TokenTree};

extern crate proc_macro;
use quote::quote as rust_quote;

#[derive(Debug)]
enum QuoteToken {
    Var(Ident),
    Content(String),
    Whitespace,
}

enum DelimiterVariant {
    Open,
    Close,
}

impl QuoteToken {
    pub fn from_delimiter(delimiter: Delimiter, variant: DelimiterVariant) -> Self {
        match (delimiter, variant) {
            (Delimiter::Brace, DelimiterVariant::Open) => Self::Content("{".to_string()),
            (Delimiter::Brace, DelimiterVariant::Close) => Self::Content("}".to_string()),
            (Delimiter::Bracket, DelimiterVariant::Open) => Self::Content("[".to_string()),
            (Delimiter::Bracket, DelimiterVariant::Close) => Self::Content("]".to_string()),
            (Delimiter::Parenthesis, DelimiterVariant::Open) => Self::Content("(".to_string()),
            (Delimiter::Parenthesis, DelimiterVariant::Close) => Self::Content(")".to_string()),
            (Delimiter::None, _) => Self::Content(String::default()),
        }
    }
}

fn process_token_stream(
    mut token_stream: Peekable<impl Iterator<Item = TokenTree>>,
    output: &mut Vec<QuoteToken>,
) {
    // Rust proc macro parser to TokenStream gets rid of all whitespaces.
    // Here we just make sure no two identifiers are without a space between them.
    let mut was_previous_ident: bool = false;
    while let Some(token_tree) = token_stream.next() {
        match token_tree {
            TokenTree::Group(group) => {
                let token_iter = group.stream().into_iter().peekable();
                let delimiter = group.delimiter();
                output.push(QuoteToken::from_delimiter(
                    delimiter,
                    DelimiterVariant::Open,
                ));
                process_token_stream(token_iter, output);
                output.push(QuoteToken::from_delimiter(
                    delimiter,
                    DelimiterVariant::Close,
                ));
                was_previous_ident = false;
            }
            TokenTree::Punct(punct) => {
                if punct.as_char() == '#' {
                    if let Some(TokenTree::Ident(ident)) = token_stream.next() {
                        let var_ident = Ident::new(&ident.to_string(), Span::call_site());
                        output.push(QuoteToken::Var(var_ident))
                    }
                } else {
                    output.push(QuoteToken::Content(punct.to_string()));
                }
                was_previous_ident = false;
            }
            TokenTree::Ident(ident) => {
                if was_previous_ident {
                    output.push(QuoteToken::Whitespace);
                }
                output.push(QuoteToken::Content(ident.to_string()));
                was_previous_ident = true;
            }
            TokenTree::Literal(literal) => {
                output.push(QuoteToken::Content(literal.to_string()));
                was_previous_ident = false;
            }
        }
    }
}

#[proc_macro]
pub fn quote(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut output_token_stream = rust_quote! {
      let mut quote_macro_result = ::cairo_lang_macro::TokenStream::empty();
    };

    let input: proc_macro2::TokenStream = input.into();
    let token_iter = input.into_iter().peekable();
    let (size_hint_lower, _) = token_iter.size_hint();
    let mut parsed_input: Vec<QuoteToken> = Vec::with_capacity(size_hint_lower);
    process_token_stream(token_iter, &mut parsed_input);

    for quote_token in parsed_input.iter() {
        match quote_token {
            QuoteToken::Content(content) => {
                output_token_stream.extend(rust_quote! {
                  quote_macro_result.push_token(::cairo_lang_macro::TokenTree::Ident(::cairo_lang_macro::Token::new(::std::string::ToString::to_string(#content), ::cairo_lang_macro::TextSpan::call_site())));
                });
            }
            QuoteToken::Var(ident) => {
                output_token_stream.extend(rust_quote! {
                  quote_macro_result.extend(::cairo_lang_macro::TokenStream::from_primitive_token_stream(::cairo_lang_primitive_token::ToPrimitiveTokenStream::to_primitive_token_stream(&#ident)).into_iter());
                });
            }
            QuoteToken::Whitespace => output_token_stream.extend(rust_quote! {
              quote_macro_result.push_token(::cairo_lang_macro::TokenTree::Ident(::cairo_lang_macro::Token::new(" ".to_string(), ::cairo_lang_macro::TextSpan::call_site())));
            }),
        }
    }
    proc_macro::TokenStream::from(rust_quote!({
      #output_token_stream
      quote_macro_result
    }))
}
