pub use cairo_lang_macro::{TokenStream, TokenTree};
pub use cairo_lang_utils::Upcast;
#[cfg(test)]
mod test;

/// Macro that works similar to rust's quote and format macro. It takes a string literal, which includes `{}` sections.
/// Like in format macro in rust, it also takes the same number of arguments as `{}` in a literal. Those values are supposed to be a type of [cairo_lang_syntax::node::SyntaxNode].
/// Macro will produce a new [TokenStream], which will persist all the TextSpan values from passed SyntaxNodes inside.
#[macro_export]
macro_rules! quote_format {
  ($literal:expr) => {{
    use $crate::{split_by_space_and_pos_arg, TokenTree, TokenStream};

    let positional_arguments: Vec<_> = $literal.matches("{}").collect();
    let positional_arguments_len = positional_arguments.len();

    assert!(positional_arguments_len == 0, "0 arguments found but the numer of positional arguments is: {}", positional_arguments_len);

    let mut result_tokens: Vec<TokenTree> = Vec::default();
    let splitted_literal = split_by_space_and_pos_arg($literal);

    for string_token in splitted_literal.into_iter() {
          // Plain code inserted into the quote won't be getting any span.
          // For tokens with no span, the macro host will decide upon what should be the default behaviour.
          result_tokens.push(TokenTree::Ident(Token::new(string_token, None)));
    }

    TokenStream::new(result_tokens)
  }};
  ($db:expr, $literal:expr, $($arg:expr),*) => {{
    // As the user writes a macro, we can assume that those packages are included inside the user's Cargo.toml.
    use cairo_lang_macro::{TokenStream, TokenTree};
    use cairo_lang_syntax::node::{SyntaxNode, db::SyntaxGroup};
    use $crate::split_by_space_and_pos_arg;

    trait Tokenable {
      fn to_tokens(&self, db: &dyn SyntaxGroup) -> Vec<TokenTree>;
    }

    impl Tokenable for TokenStream {
      fn to_tokens(&self, _db: &dyn SyntaxGroup) -> Vec<TokenTree> {
          self.tokens.clone()
      }
    }

    impl Tokenable for &SyntaxNode {
        fn to_tokens(&self, db: &dyn SyntaxGroup) -> Vec<TokenTree> {
            let node_span = self.span(db).to_str_range();
            vec![TokenTree::Ident(Token::new(
                self.get_text(db),
                Some(TextSpan::new(node_span.start, node_span.end)),
            ))]
        }
    }

    impl Tokenable for SyntaxNode {
        fn to_tokens(&self, db: &dyn SyntaxGroup) -> Vec<TokenTree> {
            let node_span = self.span(db).to_str_range();
            vec![TokenTree::Ident(Token::new(
                self.get_text(db),
                Some(TextSpan::new(node_span.start, node_span.end)),
            ))]
        }
    }

    impl Tokenable for TokenTree {
        fn to_tokens(&self, _db: &dyn SyntaxGroup) -> Vec<TokenTree> {
            vec![self.clone()]
        }
    }

    let db = $db.upcast();
    let positional_arguments: Vec<_> = $literal.matches("{}").collect();
    let positional_arguments_number = positional_arguments.len();
    let args_static = [$($arg),*];
    let args = args_static.iter().map(|tokenable| tokenable as &dyn Tokenable).collect::<Vec<_>>();
    let args_num: usize = args.len();

    assert!(
      positional_arguments_number >= args_num,
        "Too many arguments provided for the number of positional arguments. Positional arguments: {}, arguments: {}", positional_arguments_number, args_num
    );
    assert!(
      args_num >= positional_arguments_number,
        "Too many positional arguments for provided arguments. Positional arguments: {}, arguments: {}", positional_arguments_number, args_num
    );

    let mut result_tokens: Vec<TokenTree> = Vec::default();
    let splitted_literal = split_by_space_and_pos_arg($literal);
    let mut arg_index: usize = 0;

    for string_token in splitted_literal.into_iter() {
        if string_token == "{}" {
            let arg = args.get(arg_index).unwrap();
            result_tokens.extend(arg.to_tokens(db));
            arg_index += 1;
        } else {
            // Plain code inserted into the quote won't be getting any span.
            // For tokens with no span, the macro host will decide upon what should be the default behaviour.
            result_tokens.push(TokenTree::Ident(Token::new(string_token, None)));
        }
    }

    TokenStream::new(result_tokens)
}};
}

pub fn split_by_space_and_pos_arg(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut last = 0;

    let mut match_indices: Vec<_> = s.match_indices(' ').chain(s.match_indices("{}")).collect();
    match_indices.sort_by_key(|pair| pair.0);

    for (index, matched) in match_indices {
        if last != index {
            result.push(s[last..index].to_owned());
        }
        result.push(matched.to_owned());
        last = index + matched.len();
    }

    if last < s.len() {
        result.push(s[last..].to_owned());
    }
    result
}
