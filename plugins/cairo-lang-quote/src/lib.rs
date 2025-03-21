use std::iter::Peekable;

use proc_macro2::{Delimiter, Ident, Span, TokenTree};

extern crate proc_macro;
use quote::quote as rust_quote;

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
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
                    // Only peek, so items precessed with punct can be handled in next iteration.
                    if let Some(TokenTree::Ident(ident)) = token_stream.peek() {
                        if was_previous_ident {
                            output.push(QuoteToken::Whitespace);
                        }
                        let var_ident = Ident::new(&ident.to_string(), Span::call_site());
                        output.push(QuoteToken::Var(var_ident));
                        // Move iterator, as we only did peek before.
                        let _ = token_stream.next();
                    } else {
                        // E.g. to support Cairo attributes (i.e. punct followed by non-ident `#[`).
                        output.push(QuoteToken::Content(punct.to_string()));
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

#[cfg(test)]
mod tests {
    use super::{process_token_stream, QuoteToken};
    use proc_macro2::{Ident, Span};
    use quote::{quote as rust_quote, TokenStreamExt};

    #[test]
    fn parse_cairo_attr() {
        let input: proc_macro2::TokenStream = rust_quote! {
            #[some_attr]
            fn some_fun() {

            }
        };
        let mut output = Vec::new();
        process_token_stream(input.into_iter().peekable(), &mut output);
        assert_eq!(
            output,
            vec![
                QuoteToken::Content("#".to_string()),
                QuoteToken::Content("[".to_string()),
                QuoteToken::Content("some_attr".to_string()),
                QuoteToken::Content("]".to_string()),
                QuoteToken::Content("fn".to_string()),
                QuoteToken::Whitespace,
                QuoteToken::Content("some_fun".to_string()),
                QuoteToken::Content("(".to_string()),
                QuoteToken::Content(")".to_string()),
                QuoteToken::Content("{".to_string()),
                QuoteToken::Content("}".to_string()),
            ]
        );
    }

    #[test]
    fn quote_var_whitespace() {
        /*
        Construct program input, equivalent to following:
        input = rust_quote! {
            #[some_attr]
            mod #name {
            }
        }
        In a way that avoids `#name` being parsed as `rust_quote` var.
        */
        let mut input: proc_macro2::TokenStream = rust_quote! {
            #[some_attr]
            mod
        };
        input.append(proc_macro2::TokenTree::Punct(proc_macro2::Punct::new(
            '#',
            proc_macro2::Spacing::Joint,
        )));
        input.extend(rust_quote! {
            name {
            }
        });
        let mut output = Vec::new();
        process_token_stream(input.into_iter().peekable(), &mut output);
        assert_eq!(
            output,
            vec![
                QuoteToken::Content("#".to_string()),
                QuoteToken::Content("[".to_string()),
                QuoteToken::Content("some_attr".to_string()),
                QuoteToken::Content("]".to_string()),
                QuoteToken::Content("mod".to_string()),
                QuoteToken::Whitespace,
                QuoteToken::Var(Ident::new("name", Span::call_site())),
                QuoteToken::Content("{".to_string()),
                QuoteToken::Content("}".to_string()),
            ]
        );
    }
}
