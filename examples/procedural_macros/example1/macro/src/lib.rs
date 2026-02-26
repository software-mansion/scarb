use cairo_lang_macro::{
    Diagnostics, ProcMacroResult, TextSpan, Token, TokenStream, TokenTree, inline_macro,
};

#[inline_macro]
pub fn fib(args: TokenStream) -> ProcMacroResult {
    let argument = match parse_arguments(args) {
        Ok(arg) => arg,
        Err(diagnostics) => {
            return ProcMacroResult::new(TokenStream::new(vec![])).with_diagnostics(diagnostics);
        }
    };

    let result = fib(argument);

    ProcMacroResult::new(TokenStream::new(vec![TokenTree::Ident(Token::new(
        result.to_string(),
        TextSpan::call_site(),
    ))]))
}

/// Parse argument into a numerical value.
///
/// Always expects a single, numerical value in parentheses.
/// Panics otherwise.
fn parse_arguments(args: TokenStream) -> Result<u32, Diagnostics> {
    let args = args.to_string();
    let (_prefix, rest) = args
        .split_once("(")
        .ok_or_else(|| Diagnostics::new(Vec::new()).error("Invalid format: expected '('"))?;
    let (argument, _suffix) = rest
        .rsplit_once(")")
        .ok_or_else(|| Diagnostics::new(Vec::new()).error("Invalid format: expected ')'"))?;
    let argument = argument
        .parse::<u32>()
        .map_err(|_| Diagnostics::new(Vec::new()).error("Invalid argument: expected a number"))?;
    Ok(argument)
}

/// Calculate n-th Fibonacci number.
fn fib(mut n: u32) -> u32 {
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
