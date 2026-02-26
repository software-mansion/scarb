use cairo_lang_macro::{inline_macro, ProcMacroResult, TextSpan, Token, TokenStream, TokenTree};

#[inline_macro]
pub fn fib(args: TokenStream) -> ProcMacroResult {
    let argument = parse_arguments(args);
    let result = fib_impl(argument);
    ProcMacroResult::new(TokenStream::new(vec![TokenTree::Ident(Token::new(
        result.to_string(),
        TextSpan::call_site(),
    ))]))
}

/// Parse argument into a numerical value.
///
/// Always expects a single, numerical value in parentheses.
/// Panics otherwise.
fn parse_arguments(args: TokenStream) -> u32 {
    let args = args.to_string();
    let (_prefix, rest) = args.split_once('(').unwrap();
    let (argument, _suffix) = rest.rsplit_once(')').unwrap();
    let argument = argument.parse::<u32>().unwrap();
    argument
}

/// Calculate n-th Fibonacci number.
fn fib_impl(mut n: u32) -> u32 {
    let mut a: u32 = 0;
    let mut b: u32 = 1;
    while n != 0 {
        n = n - 1;
        let temp = b;
        b = a + b;
        a = temp;
    }
    a
}
