use std::iter::Peekable;

use cairo_lang_macro::{TextSpan, Token, TokenStream, TokenTree};
use cairo_lang_syntax::node::{db::SyntaxGroup, SyntaxNode};
use proc_macro::{Delimiter, Ident, TokenStream as RustTokenStream, TokenTree as RustTokenTree};
use proc_macro2::{Ident as Ident2, Span};

extern crate proc_macro;
use quote::quote as rust_quote;

#[derive(Debug)]
enum QuoteToken {
    Var(Ident),
    Content(String),
}

impl QuoteToken {
    pub fn from_delimiter(delimiter: Delimiter, is_end: bool) -> Self {
        match (delimiter, is_end) {
            (Delimiter::Brace, false) => Self::Content("{".to_string()),
            (Delimiter::Brace, true) => Self::Content("}".to_string()),
            (Delimiter::Bracket, false) => Self::Content("[".to_string()),
            (Delimiter::Bracket, true) => Self::Content("]".to_string()),
            (Delimiter::Parenthesis, false) => Self::Content("(".to_string()),
            (Delimiter::Parenthesis, true) => Self::Content(")".to_string()),
            _ => Self::Content(String::default()),
        }
    }
}

fn process_token_stream(
    mut token_stream: Peekable<impl Iterator<Item = RustTokenTree>>,
    output: &mut Vec<QuoteToken>,
) {
    while let Some(token_tree) = token_stream.next() {
        match token_tree {
            RustTokenTree::Group(group) => {
                let token_iter = group.stream().into_iter().peekable();
                let delimiter = group.delimiter();
                output.push(QuoteToken::from_delimiter(delimiter, false));
                process_token_stream(token_iter, output);
                output.push(QuoteToken::from_delimiter(delimiter, true));
            }
            RustTokenTree::Punct(punct) => {
                if punct.as_char() == '#' {
                    if let Some(RustTokenTree::Ident(ident)) = token_stream.next() {
                        output.push(QuoteToken::Var(ident))
                    }
                } else {
                    output.push(QuoteToken::Content(punct.to_string()));
                }
            }
            _ => {
                output.push(QuoteToken::Content(token_tree.to_string()));
            }
        }
    }
}

#[proc_macro]
pub fn quote(input: RustTokenStream) -> RustTokenStream {
    let mut parsed_input: Vec<QuoteToken> = Vec::new();
    let mut output_token_stream = rust_quote! {
      use cairo_lang_macro::{TokenTree, Token, TokenStream};
      use cairo_lang_stable_token::ToStableTokenStream;
      let mut quote_macro_result = TokenStream::default();
    };

    let token_iter = input.into_iter().peekable();
    process_token_stream(token_iter, &mut parsed_input);

    for quote_token in parsed_input.iter() {
        match quote_token {
            QuoteToken::Content(content) => {
                output_token_stream.extend(rust_quote! {
                  quote_macro_result.push_token(TokenTree::Ident(Token::new(#content.to_string(), None)));
                });
            }
            QuoteToken::Var(ident) => {
                let ident: Ident2 = Ident2::new(&ident.to_string(), Span::call_site());
                output_token_stream.extend(rust_quote! {
                  quote_macro_result.extend(TokenStream::from_stable_token_stream(#ident.to_stable_token_stream()));
                });
            }
        }
    }

    let result = RustTokenStream::from(rust_quote!({ #output_token_stream }));

    println!("Result: {:?}", result);

    result
}
