# Diagnostics and Errors

> [!WARNING]
> Procedural macros, by design, introduce a lot of overhead during the compilation.
> They may also be harder to maintain.
> Prefer the declarative inline macros written directly in Cairo, unless you have a specific reason to use procedural macros.
> Please see the [declarative macros chapter in Cairo Book](https://www.starknet.io/cairo-book/ch12-05-macros.html#declarative-inline-macros-for-general-metaprogramming) for more information.

> [!INFO]
> To use procedural macros, you need to have Rust toolchain (Cargo) installed on your machine.
> Please see [Rust installation guide](https://www.rust-lang.org/tools/install) for more information.

## Error locations and returning errors from macro

Informative and easy to understand error messages are crucial for the developer experience of a programming language.
Since the Scarb procedural macros enable you to extend the Cairo language semantics with your own definitions, when
writing a macro, you also become responsible for designing it with error diagnostics in mind.

Note, this section is concerned with Cairo-level errors that can happen during Cairo project compilation.
Compile and runtime errors in the Rust implementation of the macro are out of scope.
Cargo will report any compile time error in the Rust macro implementation to the user during compilation.
Any runtime error in the macro will halt the compilation abruptly and force Scarb to exit.

To better understand how we can leverage error diagnostics in procedural macros, we need to first distinguish between
three kinds of errors:

1. Errors in the Cairo code written by the user, that is copied by the macro.
2. Errors in the Cairo code returned from the macro, added by the macro logic itself, not copied from the user code.
3. Errors that are shown to the user by the macro itself, which may or may not be caused by errors in the specific line
   of user code.

The first two kinds have something in common, as in both cases the macro has returned some Cairo code as a result, but
the Cairo code returned contained an error.
In both cases, the Cairo compiler creates the error diagnostic when it tries to parse the returned Cairo code (not the
procedural macro).
Thus, we collapse them to a single section below.

### Handling errors in the returned Cairo code

### Returning errors from a procedural macro

Procedural macros can emit their own compiler diagnostics, which will be displayed as warnings / errors to the user
during the compilation.
This may be useful for informing the user about mistakes they made while using the macro, for instance, when the macro
expects a certain type of argument, but the user provided a different type.
This enables the macro author to validate macro inputs in a meaningful way, without halting the macro expansion abruptly.

This diagnostics can be created with [`Diagnostic` struct](https://docs.rs/cairo-lang-macro/latest/cairo_lang_macro/struct.Diagnostic.html) from the procedural macro API.
Diagnostics can be emitted with two levels of severity: `error` and `warning`.
Warnings are only displayed to the users, while errors also exit the compilation with a non-zero exit code without
producing any output artifacts.

Apart from the severity level, diagnostics consist of two data points: a message and a span.
The first is just a string that will be shown to the user as an error message.
The latter is optional and can be used to point the diagnostic to a specific location in the user code.
If not defined, the diagnostic will point to the call site of the macro expansion, i.e., the place where the macro was
called (e.g., the attribute that is expanded).
The span should be copied from the input token stream.

To emit created diagnostics, the [`with_diagnostics` method](https://docs.rs/cairo-lang-macro/latest/cairo_lang_macro/struct.ProcMacroResult.html#method.with_diagnostics) on `ProcMacroResult` struct can be used.

Minimal example: you can create and emit an error that will stop the compilation.

```cairo
// lib.rs
use cairo_lang_macro::{attribute_macro, Diagnostic, ProcMacroResult, TokenStream};

#[attribute_macro]
pub fn some(_args: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
    let diag = Diagnostic::error("Some error from macro.");
    ProcMacroResult::new(token_stream).with_diagnostics(diag.into())
}
```

The error will be displayed to the user as follows:

```
error[E2200]: Plugin diagnostic: Some error from macro.
 --> (..)/lib.cairo:3:1
#[some]
^^^^^^^

error: could not compile `hello_world` due to 1 previous error
```

### Full example: showing meaningful errors when parsing user arguments

When we wrote the example from [parsing token streams](./parsing), we left one complexity out: we did not
validate the user input in any meaningful way.
Now we can add proper errors for invalid user input.

To do that, we will change the `main` and `parse_argument` functions.

```rust
use cairo_lang_macro::{
    inline_macro, Diagnostic, Diagnostics, ProcMacroResult, TextSpan, Token, TokenStream, TokenTree,
};
use cairo_lang_parser::utils::SimpleParserDatabase;
use cairo_lang_syntax::node::{ast, kind::SyntaxKind, TypedSyntaxNode};

#[inline_macro]
pub fn fib(args: TokenStream) -> ProcMacroResult {
    match parse_arguments(args) {
        ParseResult::Ok(argument) => {
            let result = fib(argument);
            ProcMacroResult::new(TokenStream::new(vec![TokenTree::Ident(Token::new(
                result.to_string(),
                TextSpan::call_site(),
            ))]))
        }
        ParseResult::Diagnostics(diagnostics) => {
            ProcMacroResult::new(TokenStream::empty()).with_diagnostics(diagnostics)
        }
    }
}

///Result enum for parsing arguments.
enum ParseResult<T> {
    Ok(T),
    Diagnostics(Diagnostics),
}

/// Parse argument into a numerical value.
fn parse_arguments(args: TokenStream) -> ParseResult<u32> {
    // Initialize parser.
    let db = SimpleParserDatabase::default();
    // Parse incoming token stream.
    let (node, _diagnostics) = db.parse_token_stream_expr(&args);
    let node_span = node.span(&db);
    // Validate syntax node kind.
    // Return diagnostics if invalid.
    if node.kind(&db) != SyntaxKind::ExprParenthesized {
        let span = TextSpan::new(node_span.start.as_u32(), node_span.end.as_u32());
        let diag = Diagnostic::span_error(span, "argument mut be a single value");
        return ParseResult::Diagnostics(diag.into());
    }
    // Read parsed syntax node.
    let expr = ast::ExprParenthesized::from_syntax_node(&db, node);
    // The `.expr()` function will return the inner expression inside parentheses.
    let inner_expr = expr.expr(&db);
    let inner_expr_span = inner_expr.as_syntax_node().span(&db);
    let argument = inner_expr.as_syntax_node().get_text(&db);
    // We parse the textual argument as a numerical value.
    let Ok(argument) = argument.parse::<u32>() else {
        // If an argument is not a numerical value, return diagnostics.
        let span = TextSpan::new(inner_expr_span.start.as_u32(), inner_expr_span.end.as_u32());
        let diag = Diagnostic::span_error(span, "argument mut be u32 value");
        return ParseResult::Diagnostics(diag.into());
    };
    ParseResult::Ok(argument)
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
```

To see the error messages, we compile our code using it in an invalid way like this:

```cairo
fn main() -> u32 {
    fib!(12,16);
    fib!("abcd");
    fib!(16)
}
```

We will see the following diagnostics:

```
error[E2200]: Plugin diagnostic: argument mut be a single value
 --> (..)/lib.cairo:2:9
    fib!(12,16);
        ^^^^^^^

error[E2200]: Plugin diagnostic: argument mut be u32 value
 --> (..)/lib.cairo:3:10
    fib!("abcd");
         ^^^^^^

error: could not compile `hello_world` due to 2 previous errors
```

Note that the first diagnostic points to the whole `(12,16)` part, while the second only to `"abcd"` string.
This is controlled by the span we choose: the first one came from the syntax node that described the whole macro input,
the second one from the syntax node that described the inner expression only.

This is much nicer than the previous, panicking, implementation, which in case of invalid input would end abruptly with
much less telling panic message similar to this:

```
thread '<unnamed>' (51728626) panicked at src/lib.rs:43:44:
called `Result::unwrap()` on an `Err` value: ParseIntError { kind: InvalidDigit }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

thread '<unnamed>' (51728626) panicked at library/core/src/panicking.rs:225:5:
panic in a function that cannot unwind
```
