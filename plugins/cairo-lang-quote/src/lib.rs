use std::iter::Peekable;

use proc_macro2::{Delimiter, Ident, Span, TokenTree};

extern crate proc_macro;
use quote::quote as rust_quote;
use ra_ap_rustc_parse_format::{ParseError, ParseMode, Parser, Piece, Position};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Error, Expr, LitStr, Token, parse_macro_input};

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

fn extract_doc_comment(group: &proc_macro2::Group) -> Option<String> {
    if group.delimiter() != Delimiter::Bracket {
        return None;
    }
    let mut iter = group.stream().into_iter();
    match (iter.next(), iter.next(), iter.next(), iter.next()) {
        (
            Some(TokenTree::Ident(ident)),
            Some(TokenTree::Punct(punct)),
            Some(TokenTree::Literal(lit)),
            None,
        ) if ident == "doc" && punct.as_char() == '=' => {
            if let Ok(lit_str) = syn::parse2::<LitStr>(TokenTree::Literal(lit).into()) {
                Some(lit_str.value())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn emit_doc_comment(prefix: &str, doc_content: &str, output: &mut Vec<QuoteToken>) {
    for line in doc_content.split('\n') {
        let line = line.trim_end_matches('\r');
        output.push(QuoteToken::Content(format!("{prefix}{line}\n")));
    }
}

fn process_token_stream(
    mut token_stream: Peekable<impl Iterator<Item = TokenTree> + Clone>,
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
                    // Check for doc comments, variable interpolation, or attributes.
                    if let Some(TokenTree::Ident(ident)) = token_stream.peek() {
                        if was_previous_ident {
                            output.push(QuoteToken::Whitespace);
                        }
                        let var_ident = Ident::new(&ident.to_string(), Span::call_site());
                        output.push(QuoteToken::Var(var_ident));
                        was_previous_ident = true;
                        // Move iterator, as we only did peek before.
                        let _ = token_stream.next();
                    } else if let Some(TokenTree::Group(group)) = token_stream.peek() {
                        if let Some(doc_content) = extract_doc_comment(group) {
                            token_stream.next();
                            emit_doc_comment("///", &doc_content, output);
                            was_previous_ident = false;
                        } else {
                            output.push(QuoteToken::Content(punct.to_string()));
                            was_previous_ident = false;
                        }
                    } else if let Some(TokenTree::Punct(p)) = token_stream.peek() {
                        if p.as_char() == '!' {
                            let mut lookahead = token_stream.clone();
                            lookahead.next(); // skip '!'
                            if let Some(TokenTree::Group(group)) = lookahead.next() {
                                if let Some(doc_content) = extract_doc_comment(&group) {
                                    token_stream.next(); // consume '!'
                                    token_stream.next(); // consume group
                                    emit_doc_comment("//!", &doc_content, output);
                                    was_previous_ident = false;
                                    continue;
                                }
                            }
                        }
                        output.push(QuoteToken::Content(punct.to_string()));
                        was_previous_ident = false;
                    } else {
                        // E.g. to support Cairo attributes (i.e. punct followed by non-ident `#[`).
                        output.push(QuoteToken::Content(punct.to_string()));
                        was_previous_ident = false;
                    }
                } else {
                    output.push(QuoteToken::Content(punct.to_string()));
                    was_previous_ident = false;
                }
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

struct QuoteFormatArgs {
    fmtstr: LitStr,
    args: Punctuated<Expr, Token![,]>,
}

impl Parse for QuoteFormatArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let fmtstr = input.parse::<LitStr>()?;

        let args = if input.peek(Token![,]) {
            let _ = input.parse::<Token![,]>()?;
            Punctuated::parse_terminated(input)?
        } else {
            Punctuated::new()
        };

        Ok(QuoteFormatArgs { fmtstr, args })
    }
}

/// Basic tokenizer for `quote_format!` macro.
///
/// Intentionally simplified to avoid full parsing of Cairo syntax.
/// Only splits strings into tokens and preserves whitespace.
/// Token kinds and spans are ignored as spans always set to call site.
/// Additionally, it's expected placeholders (`{}`, `{0}`, etc.) are already
/// stripped out by the format parser before this function is called.
fn tokenize_basic(string: &str) -> Vec<QuoteToken> {
    string
        .split(|ch: char| ch.is_whitespace() && ch != '\n')
        .map(|s| QuoteToken::Content(s.to_string()))
        .flat_map(|content| [QuoteToken::Whitespace, content])
        .skip(1)
        .collect()
}

/// Build a Cairo TokenStream from a string literal with format placeholders.
///
/// Unlike `quote!` macro, this macro bypasses Rust's parser,
/// allowing Cairo-specific syntax that is not valid Rust syntax.
///
/// Unlike `quote!` macro, this macro does not support token `#token` interpolation.
/// Placeholders are substituted with arguments implementing `ToPrimitiveTokenStream`.
/// Supported format placeholders are: `{}`, `{0}`, `{1}`, etc.
#[proc_macro]
pub fn quote_format(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let QuoteFormatArgs { fmtstr, args } = parse_macro_input!(input as QuoteFormatArgs);
    let fmtsrc = fmtstr.value();
    let args: Vec<&Expr> = args.iter().collect();

    let mut output_token_stream = rust_quote! {
      let mut quote_macro_result = ::cairo_lang_macro::TokenStream::empty();
    };
    let mut parser = Parser::new(&fmtsrc, None, None, false, ParseMode::Format);

    for piece in &mut parser {
        match piece {
            Piece::Lit(string) => {
                for token in tokenize_basic(string) {
                    match token {
                        QuoteToken::Content(content) => {
                            output_token_stream.extend(rust_quote! {
                              quote_macro_result.push_token(::cairo_lang_macro::TokenTree::Ident(::cairo_lang_macro::Token::new(::std::string::ToString::to_string(#content), ::cairo_lang_macro::TextSpan::call_site())));
                            });
                        }
                        // Vars are handled via placeholders, so they should not appear here.
                        QuoteToken::Var(_) => {
                            unreachable!("tokenizer cannot return a var quote token type")
                        }
                        QuoteToken::Whitespace => {
                            output_token_stream.extend(rust_quote! {
                              quote_macro_result.push_token(::cairo_lang_macro::TokenTree::Ident(::cairo_lang_macro::Token::new(" ".to_string(), ::cairo_lang_macro::TextSpan::call_site())));
                            });
                        }
                    }
                }
            }
            Piece::NextArgument(arg) => {
                let expr = match arg.position {
                    Position::ArgumentIs(idx) | Position::ArgumentImplicitlyIs(idx) => {
                        if let Some(expr) = args.get(idx).copied() {
                            expr
                        } else {
                            return Error::new(
                                fmtstr.span(),
                                format!(r#"format arg index {} is out of range (the format string contains {} args)."#,
                                idx,
                                args.len()
                                )
                            )
                                .to_compile_error()
                                .into();
                        }
                    }
                    Position::ArgumentNamed(name) => {
                        return Error::new(
                            fmtstr.span(),
                            format!(
                                "named placeholder '{{{}}}' is not supported by this macro.\nhelp: use positional ('{{}}') or indexed placeholders ('{{0}}', '{{1}}', ...) instead.",
                                name
                            ),
                        )
                        .to_compile_error()
                        .into();
                    }
                };
                output_token_stream.extend(rust_quote! {
                  quote_macro_result.extend(
                    ::cairo_lang_macro::TokenStream::from_primitive_token_stream(::cairo_lang_primitive_token::ToPrimitiveTokenStream::to_primitive_token_stream(&#expr)).into_iter()
                  );
                });
            }
        }
    }
    if !parser.errors.is_empty() {
        let ParseError {
            description,
            note,
            label,
            span: _,
            secondary_label: _,
            suggestion: _,
        } = parser.errors.remove(0);
        let mut err_msg = format!("failed to parse format string: {label}\n{description}");
        if let Some(note) = note {
            err_msg.push_str(&format!("\nnote: {note}"));
        }
        return Error::new(fmtstr.span(), err_msg).to_compile_error().into();
    }
    proc_macro::TokenStream::from(rust_quote!({
      #output_token_stream
      quote_macro_result
    }))
}

#[cfg(test)]
mod tests {
    use super::{QuoteToken, process_token_stream};
    use proc_macro2::{Ident, Span};
    use quote::{TokenStreamExt, quote as rust_quote};

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

    #[test]
    fn interpolate_tokens() {
        use super::{QuoteToken, process_token_stream};
        use proc_macro2::{Ident, Punct, Spacing, Span, TokenTree};
        use quote::{TokenStreamExt, quote as rust_quote};

        // impl #impl_token of NameTrait<#name_token> {}

        let mut input: proc_macro2::TokenStream = rust_quote! {
            impl
        };
        input.append(TokenTree::Punct(Punct::new('#', Spacing::Joint)));
        input.extend(rust_quote! {
            impl_token
        });
        input.extend(rust_quote! {
            of NameTrait<
        });
        input.append(TokenTree::Punct(Punct::new('#', Spacing::Joint)));
        input.extend(rust_quote! {
            name_token> {}
        });

        let mut output = Vec::new();
        process_token_stream(input.into_iter().peekable(), &mut output);
        assert_eq!(
            output,
            vec![
                QuoteToken::Content("impl".to_string()),
                QuoteToken::Whitespace,
                QuoteToken::Var(Ident::new("impl_token", Span::call_site())),
                QuoteToken::Whitespace,
                QuoteToken::Content("of".to_string()),
                QuoteToken::Whitespace,
                QuoteToken::Content("NameTrait".to_string()),
                QuoteToken::Content("<".to_string()),
                QuoteToken::Var(Ident::new("name_token", Span::call_site())),
                QuoteToken::Content(">".to_string()),
                QuoteToken::Content("{".to_string()),
                QuoteToken::Content("}".to_string()),
            ]
        );
    }

    #[test]
    fn single_line_doc_comment() {
        let input: proc_macro2::TokenStream = rust_quote! {
            /// This is a doc comment.
            fn some_fun() {}
        };
        let mut output = Vec::new();
        process_token_stream(input.into_iter().peekable(), &mut output);
        assert_eq!(
            output,
            vec![
                QuoteToken::Content("/// This is a doc comment.\n".to_string()),
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
    fn multi_line_doc_comment() {
        let input: proc_macro2::TokenStream = rust_quote! {
            /// First line.
            /// Second line.
            ///
            /// Fourth line.
            fn some_fun() {}
        };
        let mut output = Vec::new();
        process_token_stream(input.into_iter().peekable(), &mut output);
        assert_eq!(
            output,
            vec![
                QuoteToken::Content("/// First line.\n".to_string()),
                QuoteToken::Content("/// Second line.\n".to_string()),
                QuoteToken::Content("///\n".to_string()),
                QuoteToken::Content("/// Fourth line.\n".to_string()),
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
    fn inner_doc_comment() {
        let input: proc_macro2::TokenStream = rust_quote! {
            //! Inner module doc comment.
            //! Second inner line.
            mod my_mod {}
        };
        let mut output = Vec::new();
        process_token_stream(input.into_iter().peekable(), &mut output);
        assert_eq!(
            output,
            vec![
                QuoteToken::Content("//! Inner module doc comment.\n".to_string()),
                QuoteToken::Content("//! Second inner line.\n".to_string()),
                QuoteToken::Content("mod".to_string()),
                QuoteToken::Whitespace,
                QuoteToken::Content("my_mod".to_string()),
                QuoteToken::Content("{".to_string()),
                QuoteToken::Content("}".to_string()),
            ]
        );
    }

    #[test]
    fn doc_comment_with_regular_attributes() {
        let input: proc_macro2::TokenStream = rust_quote! {
            /// Function doc.
            #[derive(Drop)]
            #[inline]
            fn some_fun() {}
        };
        let mut output = Vec::new();
        process_token_stream(input.into_iter().peekable(), &mut output);
        assert_eq!(
            output,
            vec![
                QuoteToken::Content("/// Function doc.\n".to_string()),
                QuoteToken::Content("#".to_string()),
                QuoteToken::Content("[".to_string()),
                QuoteToken::Content("derive".to_string()),
                QuoteToken::Content("(".to_string()),
                QuoteToken::Content("Drop".to_string()),
                QuoteToken::Content(")".to_string()),
                QuoteToken::Content("]".to_string()),
                QuoteToken::Content("#".to_string()),
                QuoteToken::Content("[".to_string()),
                QuoteToken::Content("inline".to_string()),
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
    fn inner_regular_attributes_untouched() {
        let input: proc_macro2::TokenStream = rust_quote! {
            #![some_inner_attr]
            mod my_mod {}
        };
        let mut output = Vec::new();
        process_token_stream(input.into_iter().peekable(), &mut output);
        assert_eq!(
            output,
            vec![
                QuoteToken::Content("#".to_string()),
                QuoteToken::Content("!".to_string()),
                QuoteToken::Content("[".to_string()),
                QuoteToken::Content("some_inner_attr".to_string()),
                QuoteToken::Content("]".to_string()),
                QuoteToken::Content("mod".to_string()),
                QuoteToken::Whitespace,
                QuoteToken::Content("my_mod".to_string()),
                QuoteToken::Content("{".to_string()),
                QuoteToken::Content("}".to_string()),
            ]
        );
    }

    #[test]
    fn doc_comment_inside_struct() {
        let input: proc_macro2::TokenStream = rust_quote! {
            struct MyStruct {
                /// Field doc.
                x: felt252,
            }
        };
        let mut output = Vec::new();
        process_token_stream(input.into_iter().peekable(), &mut output);
        assert_eq!(
            output,
            vec![
                QuoteToken::Content("struct".to_string()),
                QuoteToken::Whitespace,
                QuoteToken::Content("MyStruct".to_string()),
                QuoteToken::Content("{".to_string()),
                QuoteToken::Content("/// Field doc.\n".to_string()),
                QuoteToken::Content("x".to_string()),
                QuoteToken::Content(":".to_string()),
                QuoteToken::Content("felt252".to_string()),
                QuoteToken::Content(",".to_string()),
                QuoteToken::Content("}".to_string()),
            ]
        );
    }
}
