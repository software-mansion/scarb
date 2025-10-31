<script setup>
import { data as rel } from "../../github.data";
</script>

# Procedural Macros

> [!WARNING]
> Procedural macros, by design, introduce a lot of overhead during the compilation.
> They may also be harder to maintain.
> Prefer the declarative inline macros written directly in Cairo, unless you have a specific reason to use procedural macros.
> Please see the [declarative macros chapter in Cairo Book](https://www.starknet.io/cairo-book/ch12-05-macros.html#declarative-inline-macros-for-general-metaprogramming) for more information.

> [!INFO]
> To use procedural macros, you need to have Rust toolchain (Cargo) installed on your machine.
> Please see [Rust installation guide](https://www.rust-lang.org/tools/install) for more information.

## Summary

Inspired by Rust's procedural macro system, Scarb procedural macros aim to bring user-defined macros support to Cairo
packages.
In general, this allows writing expressions (`macro!()`), attributes (`#[macro]`), and derive
macros (`#[derive(Macro)]`) that transform Cairo code in your package.
These transformations can be distributed as packages and loaded dynamically as package dependencies.

### Procedural macro API interface

<BigLink href="https://docs.rs/cairo-lang-macro">
Go to cairo-lang-macro documentation on docs.rs
</BigLink>

## Procedural macros overview

### Procedural macro user perspective

To use a procedural macro, a Cairo programmer needs to:

- Declare a dependency on a package, that implements the procedural macro, by adding it to the `dependencies` section in
  the Scarb manifest file.
- Use the procedural macro in Cairo code, by calling it, or adding an attribute or derive macro to a Cairo item.

Since Scarb procedural macros are, in fact, Rust functions that are usually distributed as source code and compiled into
shared libraries ([see writing a procedural macro for more details](#writing-a-procedural-macro)) on the user side,
users are **required to have Rust toolchain installed** on their machine.
This limitation can be omitted by distributing procedural macros as precompiled shared libraries, see
the [prebuilt procedural macros](#prebuilt-procedural-macros) section for more details.

Apart from this requirement, the user does not have to perform any additional steps to use a procedural macro.
In particular, these two steps can be performed without any knowledge of Rust, or even the fact that the procedural
macro is implemented in Rust.

Specifically, the following points are true:

#### Procedural macro packages can be used as dependencies

- Scarb packages can simply declare dependency relationships on other packages that implement Cairo procedural macros.
- Because of the semantics of Scarb package resolution, it will guarantee by itself that only one instance of a given
  procedural macro package exists in the resolved package set.
  - In other words, Scarb will out of the box verify that there is no simultaneous dependency on `proc-macro 1.0.0`
    and `proc-macro 2.0.0` or `proc-macro 1.0.1`.
- Procedural macros will end up being actual Scarb compilation unit components, though, because they will have to be
  treated differently from regular components, they will not be listed under `components` fields, but rather in a new
  one: `plugins`.

#### Procedural macro must be called from Cairo code

The procedural macro has to be called from Cairo code to be executed during the compilation.

The procedural macro will be triggered by one of three Cairo expressions

- Macro call, e.g. `macro!`
- Macro attribute, e.g. `#[macro]`
- Macro derive, e.g. `#[derive(Macro)]`

**Example:**

Scarb manifest file:

```toml
[package]
name = "hello-macros"
version = "0.1.0"

[dependencies]
add-macro = "0.1.0"
tracing-macro = "0.1.0"
to-value-macro = "0.1.0"
```

Cairo source file:

```cairo
#[derive(ToValue)]
struct Input {
    value: felt252,
}

#[instrument]
fn main() -> felt252 {
    let a = Input { value: 1 };
    let b = Input { value: 2 };
    add!(a.to_value(), b.to_value());
}
```

## Writing a procedural macro

Scarb procedural macros are, in fact, Rust functions that take **Cairo code as input** and **return modified Cairo**
code as an output.

A procedural macro is implemented as a Rust library which defines functions that implement these transformations
(later called macro _expansions_).
This Rust code is then compiled into a shared library (shared object) and loaded into Scarb process memory during the
Cairo project compilation.
Scarb will call expansions from the loaded shared library, thus allowing you to inject custom logic to the Cairo
compilation process.

### Procedural macro author perspective

To implement a procedural macro, a programmer has to:

- Create a new package, with a `Scarb.toml` manifest file, `Cargo.toml` manifest file and a `src/` directory besides.
- The Scarb manifest file must define a `cairo-plugin` target type.
- The Cargo manifest file must define a `crate-type = ["cdylib"]` on `[lib]` target.
- Write a Rust library, inside the `src/` directory that implements the procedural macro API.
- A Rust crate exposing an API for writing procedural macros is published for programmers under the
  name `cairo-lang-macro`. This crate must be added to the `Cargo.toml` file.
- The Rust library contained in the package have to implement a function responsible for code expansion.
- This function accepts a `TokenStream` as an input and returns a `ProcMacroResult` as an output, both defined in the
  helper library.
- The result struct contains the transformed `TokenStream`. Three kinds of results are possible:
  - If the `TokenStream` is the same as the input, the AST is not modified.
  - If the `TokenStream` is different from the input, the input is replaced with the generated code.
  - If the `TokenStream` is empty, the input is removed.
- Alongside the new TokenStream, a procedural macro can emit compiler diagnostics, auxiliary data and full path
  identifiers, described in detail in advanced macro usage section.

We define `TokenStream` as some encapsulation of code represented in plain Cairo.

### Creating procedural macros with helpers

The API for writing procedural macros for Cairo is defined in the `cairo-lang-macro` crate.
This interface includes both structures shared between the procedural macro and Scarb, as well as a set of helper macros
that hide the details of the FFI communication from the procedural macro author.

These three macro helpers are:

1. `#[inline_macro]` - Implements an expression macro. Should be used on function that accepts single token stream.
2. `#[attribute_macro]` - Implements an attribute macro. Should be used on function that accepts two token streams -
   first for the attribute arguments (`#[macro(arguments)]`) and second for the item the attribute is applied to.
3. `#[derive_macro]` - Implements a derive macro. Should be used on function that accepts single token stream, the item
   the derive is applied to. Note that derives cannot replace the original item, but rather add new items to the module.

You can find documentation for these helpers in [attribute macros section](https://docs.rs/cairo-lang-macro/latest/cairo_lang_macro/#attributes)
of the `cairo-lang-macro` crate documentation.

#### First example: a macro that removes code

For example, you can implement a primitive procedural macro, which acts as an attribute that removes whatever code is
marked with it.

```toml
# Scarb.toml
[package]
name = "remove_item_macro"
version = "0.1.0"

[cairo-plugin]
```

```toml-vue
# Cargo.toml
[package]
name = "remove_item_macro"
version = "0.1.0"
edition = "2021"
publish = false

[lib]
crate-type = ["cdylib"]

[dependencies]
cairo-lang-macro = "{{ rel.stable.cairoLangMacroVersion }}"
```

```rust
// src/lib.rs
use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro};

#[attribute_macro]
pub fn remove_item(_args: TokenStream, _body: TokenStream) -> ProcMacroResult {
    ProcMacroResult::new(TokenStream::empty())
}
```

You can test this macro by annotating some function with it:

```toml
# hello_world/Scarb.toml
[package]
name = "hello_world"
version = "0.1.0"
edition = "2023_10"

[dependencies]
remove_item_macro = { path = "../remove_item_macro" }
```

```cairo
// hello_world/src/lib.cairo
fn main() -> u32 {
    fib(16)
}

#[remove_item]
fn fib(mut n: u32) -> u32 {
    let mut a: u32 = 0;
    let mut b: u32 = 1;
    while n != 0 {
        n = n - 1;
        let temp = b;
        b = a + b;
        a = temp;
    };
    a
}
```

And the compilation will fail with following error:

```sh
Compiling hello_world v0.1.0 (../hello_world/Scarb.toml)
error[E0006]: Function not found.
 --> ../hello_world/src/lib.cairo:2:5
    fib(16)
    ^^^

error: could not compile `hello_world` due to previous error
```

Maybe it's not the most productive code you wrote, but the function has been removed during the compilation!

#### Second example: returning a value

Note, we omit the toml files here, as their content is the same as in the previous example.

Usually you want to define a procedural macro that injects some code into your Cairo project.
In this example, we will create an inline procedural macro that returns a single numerical value as a token.

```rust
use cairo_lang_macro::{inline_macro, ProcMacroResult, TextSpan, Token, TokenStream, TokenTree};

#[inline_macro]
pub fn fib(args: TokenStream) -> ProcMacroResult {
    let argument = parse_arguments(args);

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
fn parse_arguments(args: TokenStream) -> u32 {
    let args = args.to_string();
    let (_prefix, rest) = args.split_once("(").unwrap();
    let (argument, _suffix) = rest.rsplit_once(")").unwrap();
    let argument = argument.parse::<u32>().unwrap();
    argument
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

This example is a bit more complex than the previous one.
The macro works in three steps:

1. Parse inline macro arguments.
2. Perform some computation in Rust.
3. Construct and return a new `TokenStream` as a result.

The first step is done by the `parse_arguments` function, in a very primitive way.
We convert the whole input `TokenStream` into a single string and then look for left and right parentheses.
We always assume the argument to be a single numerical value.

> [!WARNING]
> This function is only useful for demonstration.
> In reality, you should make your parser more robust and never should assume that the input is valid.
> Properly handling parsing errors is a must if you want your users to understand why their code is not compiling.
> Please see [parsing token stream](#parsing-token-streams) for more information.

We then call the `fib` function, which calculates a number in Fibonacci sequence.
Note that this calculation happens **during the compilation**, when the procedural macro expansion happens, not during
the Cairo program execution.

The result is a single numerical value, that we convert to a `TokenStream`, by wrapping it in three subsequent abstractions:
`Token`, `TokenTree` and `TokenStream`.
`Token` represents a single Cairo token, and consists of two parts: a string representing the token content and a span.
Span is a location in the source code of a project that uses this macro.
It is used to persist information about the origin of tokens that are moved or copied from user code.
For new tokens, that you create in your macro like we do here, it should be set to `TextSpan::call_site()`, which is
a span that points to the location of the macro call.
`TokenTree` is an additional enum that describes the type of the token, currently only `TokenTree::Ident` is used (but
may be more in the future).
Finally, `TokenStream` is a stream of `TokenTree`s, that can be iterated over or converted into a string.

Then you can use this macro in your Cairo code:
Note that `fib!(16)` actually calls the `fib` inline macro we defined before.

```cairo
fn main() -> u32 {
    fib!(16)
}

#[cfg(test)]
mod tests {
    use super::main;

    #[test]
    fn it_works() {
        assert(main() == 987, 'it works!');
    }
}
```

If you test your program with `scarb test`, it works:

```
Collected 1 test(s) from hello_world package
Running 1 test(s) from src/
[PASS] hello_world::tests::it_works (l1_gas: ~0, l1_data_gas: ~0, l2_gas: ~40000)
Tests: 1 passed, 0 failed, 0 ignored, 0 filtered out
```

Notice how no computations actually happen during Cairo program execution.
This Cairo project compiles into the following CASM code:

```
[ap + 0] = 987, ap++;
ret;
```

> [!INFO]
> To see a real life example of a procedural macro that offloads some work into compile time,
> you can take a look at the [`alexandria` project](https://github.com/keep-starknet-strange/alexandria/tree/6b98da52c819aeb86697b787b4bcf4abe94bc788/packages/macros).

### Parsing token streams

We said before, that `TokenStream` is some encapsulation of code represented in plain Cairo.
To use procedural macro arguments passed in this format, you need to parse them semantically in some way.

For simple use cases, like the one we used in the previous example, you can resort to simple methods like writing
your own parsers by hand or using regex expressions. However, the more complex the code you want to parse (and accept as
your macro input), the more efficient and robust your solution needs to be.

Instead of reimplementing a Cairo parser on your own, you can use the one defined for the Cairo compiler.
This parser can be found in `cairo-lang-parser` crate available on crates.io package registry, and it's source code is
part of the [Cairo compiler repository](https://github.com/starkware-libs/cairo).

To use the parser, you need to initialize it first.
The `SimpleParserDatabase` struct implements a convenient wrapper around the parser that will do the setup for you - just
write `let db = SimpleParserDatabase::default()`.
Then, you can use the [`parse_token_stream_expr`](https://docs.rs/cairo-lang-parser/latest/cairo_lang_parser/utils/struct.SimpleParserDatabase.html#method.parse_token_stream_expr)
function to parse single Cairo expression, or the [`parse_token_stream`](https://docs.rs/cairo-lang-parser/latest/cairo_lang_parser/utils/struct.SimpleParserDatabase.html#method.parse_token_stream)
function to parse full Cairo statements.

Example:

We will modify the previous example, so it uses the Cairo parser to parse the macro arguments.
We first add three dependencies to our project:

- [`cairo-lang-parser`](https://crates.io/crates/cairo-lang-parser) - the Cairo parser itself
- [`cairo-lang-syntax`](https://crates.io/crates/cairo-lang-syntax) - crate that defines the Cairo [abstract syntax tree (AST)](https://en.wikipedia.org/wiki/Abstract_syntax_tree)
  , i.e. the format that parser will parse into
- [`cairo-lang-primitive-token`](https://crates.io/crates/cairo-lang-primitive-token) - a crate that defines a common interface between the `TokenStream` from `cairo-lang-macro`
  and something that the parser can understand. This is a technicality that allows us to use a completely separate versioning
  scheme for these two crates.

```toml-vue
# Cargo.toml
[package]
name = "remove_item_macro"
version = "0.1.0"
edition = "2021"
publish = false

[lib]
crate-type = ["cdylib"]

[dependencies]
cairo-lang-macro = "{{ rel.stable.cairoLangMacroVersion }}"
cairo-lang-parser = "{{ rel.stable.cairoVersion }}"
cairo-lang-primitive-token = "1"
cairo-lang-syntax = "{{ rel.stable.cairoVersion }}"
```

We then rewrite the `parse_arguments` function.

```rust
use cairo_lang_macro::{inline_macro, ProcMacroResult, TextSpan, Token, TokenStream, TokenTree};
use cairo_lang_parser::utils::SimpleParserDatabase;
use cairo_lang_syntax::node::{ast, kind::SyntaxKind, TypedSyntaxNode};

/// Parse argument into a numerical value.
///
/// Always expects a single, numerical value in parentheses.
/// Panics otherwise.
fn parse_arguments(args: TokenStream) -> u32 {
    // Initialize parser.
    let db = SimpleParserDatabase::default();
    // Parse incoming token stream.
    let (node, _diagnostics) = db.parse_token_stream_expr(&args);
    // Read parsed syntax node.
    assert_eq!(node.kind(&db), SyntaxKind::ExprParenthesized);
    let expr = ast::ExprParenthesized::from_syntax_node(&db, node);
    // The `.expr()` function will return the inner expression inside parentheses.
    let inner_expr = expr.expr(&db);
    let argument = inner_expr.as_syntax_node().get_text(&db);
    // We parse the textual argument as a numerical value.
    let argument = argument.parse::<u32>().unwrap();
    argument
}
```

> [!WARNING]
> For performance reasons, the parser depends on the spans associated with tokens in the token stream it parses.
> It requires the input to be a sequence of tokens associated with a consecutive origin, with no gaps in between.
> It also requires the sequence to start at an origin with zero spans.
>
> This means that while **Scarb guarantees all token streams passed to the procedural macro as arguments can be safely
> parsed** with the Cairo parser, generally **you should not use the parser on token streams you create**.

### Building token stream with `quote!` macro

In our macro, we manually construct the token stream we return.
This approach is fine for basic and very short results, like the single numerical value we return, but it does
not scale very well for longer results.
Constructing longer token streams this way, say a whole new function you want to return, would not be very convenient.

The `cairo-lang-macro` crate defines a [`quote!` macro](https://docs.rs/cairo-lang-macro/latest/cairo_lang_macro/macro.quote.html),
which can be used to build `TokenStream`s from Rust variables.
This acts as a convenient wrapper around creating and pushing tokens into a `TokenStream` manually.

For instance, if we decide we no longer want to return a single value from our macro, but rather create a const variable
declaration with it, we can use the `quote!` macro to make our implementation more concise.

We first change how we use our macro. The `main` function now returns `FIB16` constant, that will be later created by
the macro expansion. We move the macro call to the top level of the module.

```cairo
fib!(16);

fn main() -> u32 {
    FIB16
}
```

We also change the `fib` function to use the `quote!` macro.
Inside the macro call, we declare the constant value as if it was a normal Cairo source file.
When we want to substitute some Rust variable with its value, we can use its name prefixed with a hash sign `#`.

We can do this with any variable that implements [`ToPrimitiveTokenStream`](https://docs.rs/cairo-lang-primitive-token/latest/cairo_lang_primitive_token/trait.ToPrimitiveTokenStream.html)
trait from `cairo-lang-primitive-token` crate.
This trait is implemented for `TokenStream` itself, so we can use `quote!` for composition of multiple token streams.

```rust
#[inline_macro]
pub fn fib(args: TokenStream) -> ProcMacroResult {
    let argument = parse_arguments(args);

    let result = fib(argument);

    let result = TokenTree::Ident(Token::new(result.to_string(), TextSpan::call_site()));

    ProcMacroResult::new(quote! {
        const FIB16: u32 =  #result;
    })
}
```

In a similar manner, you can use syntax nodes from the `cairo-lang-syntax` AST as variables in the z macro.
This is especially useful when you need to copy some Cairo code from the input token stream, say, some function annotated
with your attribute procedural macro.

```rust
use cairo_lang_macro::{attribute_macro, quote, ProcMacroResult};

#[attribute_macro]
fn attr_name() {
    // Parse incoming token stream.
    let db = SimpleParserDatabase::default();
    let (node, _diagnostics) = db.parse_token_stream(&body);
    // Create `SyntaxNodeWithDb`, from a single syntax node.
    // This struct implements `ToPrimitiveTokenStream` trait, thus can be used as argument to `quote!`.
    let node = SyntaxNodeWithDb::new(&node, &db);
    // Use the node in `quote!` macro.
    ProcMacroResult::new(quote! {
        #node
    })
}
```

#### Third example: creating a new function

Working example of this approach can be an attribute macro that creates a new function wrapper.
This new function will call the original function with some argument.
The name of the wrapper function and argument value will be controlled by attribute macro arguments.

```rust
// src/lib.rs
use cairo_lang_macro::{
    attribute_macro, quote, ProcMacroResult, TextSpan, Token, TokenStream, TokenTree,
};
use cairo_lang_parser::utils::SimpleParserDatabase;
use cairo_lang_syntax::node::{
    ast::{self, ModuleItem},
    helpers::HasName,
    kind::SyntaxKind,
    with_db::SyntaxNodeWithDb,
    SyntaxNode, Terminal, TypedSyntaxNode,
};

#[attribute_macro]
fn create_wrapper(args: TokenStream, body: TokenStream) -> ProcMacroResult {
    // Initialize parser to parse function body.
    let db = SimpleParserDatabase::default();
    // Define small helper for creating single token.
    let new_token = |content| TokenTree::Ident(Token::new(content, TextSpan::call_site()));
    // Parse attribute arguments with helper function.
    let (wrapper_name, argument_value) = parse_arguments(&db, args);
    let wrapper_name = new_token(wrapper_name);
    let argument_value = new_token(argument_value);
    // Parse incoming token stream.
    let (node, _diagnostics) = db.parse_token_stream(&body);
    // Parse function name.
    let function_name = parse_function_name(&db, node.clone());
    let function_name = new_token(function_name);
    // Create `SyntaxNodeWithDb`, from a single syntax node.
    // This struct implements `ToPrimitiveTokenStream` trait, thus can be used as argument to `quote!`.
    let node = SyntaxNodeWithDb::new(&node, &db);
    ProcMacroResult::new(quote! {
        #node

        fn #wrapper_name() -> u32 {
            #function_name(#argument_value)
        }
    })
}

fn parse_function_name<'db>(db: &'db SimpleParserDatabase, node: SyntaxNode<'db>) -> String {
    assert_eq!(node.kind(db), SyntaxKind::SyntaxFile);
    let file = ast::SyntaxFile::from_syntax_node(db, node);
    let items = file.items(db).elements_vec(db);
    assert_eq!(items.len(), 1);
    let func = items.into_iter().next().unwrap();
    assert!(matches!(func, ModuleItem::FreeFunction(_)));
    let ModuleItem::FreeFunction(func) = func else {
        panic!("not a function");
    };
    func.name(db).text(db).to_string(db)
}

fn parse_arguments(db: &SimpleParserDatabase, args: TokenStream) -> (String, String) {
    // Parse argument token stream.
    let (node, _diagnostics) = db.parse_token_stream_expr(&args);
    // Read parsed syntax node.
    assert_eq!(node.kind(db), SyntaxKind::ExprListParenthesized);
    let expr = ast::ExprListParenthesized::from_syntax_node(db, node);
    // `expressions` returns a list of all expressions inside parentheses.
    // We expect two expressions, the first one is the wrapper name, the second one is the argument value.
    let mut expressions = expr.expressions(db).elements_vec(db).into_iter();
    let wrapper_name_expr = expressions.next().unwrap();
    let wrapper_name = wrapper_name_expr.as_syntax_node().get_text(db).to_string();
    let value_expr = expressions.next().unwrap();
    let value = value_expr.as_syntax_node().get_text(db).to_string();
    // We return both expressions as strings.
    (wrapper_name, value)
}
```

We can use the new attribute to generate a wrapper for our `fib` function.

```cairo
// hello_world/src/lib.cairo

fn main() -> u32 {
    named_wrapper()
}

#[create_wrapper(named_wrapper,16)]
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

#[cfg(test)]
mod tests {
    use super::main;

    #[test]
    fn it_works() {
        assert(main() == 987, 'it works!');
    }
}
```

Our test will ensure that the wrapper function can be called and returns the correct value.

```
Collected 1 test(s) from hello_world package
Running 1 test(s) from src/
[PASS] hello_world::tests::it_works (l1_gas: ~0, l1_data_gas: ~0, l2_gas: ~80000)
Tests: 1 passed, 0 failed, 0 ignored, 0 filtered out
```

### Error locations and returning errors from macro

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

#### Handling errors in the returned Cairo code

#### Returning errors from a procedural macro

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
error: Plugin diagnostic: Some error from macro.
 --> (..)/lib.cairo:3:1
#[some]
^^^^^^^

error: could not compile `hello_world` due to 1 previous error
```

#### Full example: showing meaningful errors when parsing user arguments

When we wrote the example from [parsing token streams](#parsing-token-streams), we left one complexity out: we did not
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
error: Plugin diagnostic: argument mut be a single value
 --> (..)/lib.cairo:2:9
    fib!(12,16);
        ^^^^^^^

error: Plugin diagnostic: argument mut be u32 value
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

### Macros and incremental compilation: invalidating caches with fingerprints

Scarb implements incremental caching, which means that subsequent builds can be sped up with use of caches produced
during former builds.

This is possible because the relation between Cairo code and produced artifacts is **deterministic**.
During the compilation we can save some state of the compiler at some point in time and then load it in another run
from disk and continue, as if we never stopped compiling.

As procedural macros can inject additional logic defined by the macro author, it needs to uphold the same determinism
assumptions as the compiler itself.

> [!WARNING]
> This means that **all macro outputs** should be **deterministic** in regard to **the macro input passed by Scarb**
> (i.e. the token stream the macro implementation receives as an argument).

If your macro needs to read inputs from other sources that Scarb is not aware of, say from environmental variables,
you need to define a _fingerprint_ for this input with [the fingerprint attribute](https://docs.rs/cairo-lang-macro/latest/cairo_lang_macro/attr.fingerprint.html)
from procedural macro API.
Fingerprint is a function that returns a single `u64` value.
If the value changes, Scarb will invalidate incremental caches for code depending on this macro.
This enables the macro author to manually invalidate caches based on external inputs.
Usually, this is simply a hash of the input (note that you need to use a stable hash function, like `xxh3`, not
rng-seeded ones, like the default hasher used in Rust).

## Advanced macro topics and use cases

### Prebuilt procedural macros

By default, all procedural macros are compiled on the user system before being used by Scarb.
This means that programmers that wanted to depend on a package utilizing a procedural macro have to install Rust compiler
(and Cargo) on their system.
Prebuilt macros is an opt-in feature, that enables the user to request a pre-compiled procedural macro to be used instead
of compiling it on their system themselves.

For this to be possible, two conditions need to be met:

- The procedural macro package has to be published with the precompiled macros included.
- Usage of the precompiled macro binaries needs to be explicitly allowed in the top-level Scarb toml manifest file.

To include a precompiled macro binary in your package, you need to place the binary files in `target/scarb/cairo-plugin`
directory of the package, with names adhering to the following convention: `{package_name}_v{version}_{target_name}.{dll_extension}`,
where target name describes the target OS in [Cargo conventions](https://doc.rust-lang.org/rustc/platform-support.html#tier-1-with-host-tools).
For publishing, [the `include` field](./manifest.md#include) of the package manifest may be useful, as it can be used
to instruct Scarb to include this directory when packaging Scarb package with `scarb package`/`scarb publish`.

To allow usage of precompiled procedural macros, you need to add a list of package names under `allow-prebuilt-plugins`
name in the `[tool.scarb]` section of Scarb manifest of the compiled (top-level) package.
Only the section defined in the top level package will be considered, and sections defined in dependencies will be ignored.
Note this allowlist works recursively, so adding a package name allows usage of precompiled macros in the dependency
tree of this package.

```toml
[tool.scarb]
allow-prebuilt-plugins = ["snforge_std"]
```

The prebuilt binaries are used in a best-effort manner - if it's not possible to load a prebuilt binary for any reason,
it will attempt to compile the macro source code instead.
No errors will be emitted if the prebuilt binary is not found or cannot be loaded.

### Token stream metadata

As defined before, token stream is an encapsulation of Cairo code, that can be converted into a string.
Additionally, token stream passed to the procedural macro contains metadata about the fragment of Code received.
This metadata is represented by the `TokenStreamMetadata` struct, which contains the following fields:

- `original_file_path` - The path to the file in users filesystem, from which the Cairo code was read.
- `file_id` - An identifier assigned to the file by Scarb. This identifier is guaranteed to uniquely identify file
  across all files in the Scarb project.

All fields in metadata struct are optional, but will be present in the token stream you receive from Scarb for
expansion.

This metadata can be obtained by calling `.metadata()` method on `TokenStream` struct.

### Auxiliary data

Alongside the new TokenStream, a procedural macro can emit auxiliary data, encoded as an arbitrary JSON.

```rust
use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, AuxData, PostProcessContext, post_process};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct SomeMacroDataFormat {
    msg: String
}

#[attribute_macro]
pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
    let value = SomeMacroDataFormat { msg: "Hello from some macro!".to_string() };
    let value = serde_json::to_string(&value).unwrap();
    let value: Vec<u8> = value.into_bytes();
    let aux_data = AuxData::new(value);
    ProcMacroResult::new(token_stream).with_aux_data(aux_data)
}
```

This auxiliary data can be then consumed by a post-process callback defined within the procedural macro package, which
will be executed as the last step of the project build, after the Cairo code is compiled.
Your procedural macro can defined multiple post-process callbacks, in which case they all will be executed in an
undefined order.

```rust
#[post_process]
pub fn callback(context: PostProcessContext) {
    let aux_data = context.aux_data.into_iter()
        .map(|aux_data| {
            let value: Vec<u8> = aux_data.into();
            let aux_data: SomeMacroDataFormat = serde_json::from_slice(&value).unwrap();
            aux_data
        })
        .collect::<Vec<_>>();
    println!("{:?}", aux_data);
}
```

## Appendix: Procedural macros design details

### Procedural macros are special Scarb packages containing Rust code

- Procedural macros are packaged as special Scarb packages, which use a native target type: `cairo-plugin`.
- The procedural macro package will contain Rust source code, which will be shipped to users on Scarb project
  resolution through Scarb dependencies system (same as regular packages).
- The procedural macro source code will be compiled on Scarb users system only.
- Enabling this target means that the package does not contain any Cairo code.
- This target is _exclusive_:
  - It blocks defining other targets for the package, not even `lib`.
  - It will also not be possible to declare dependencies, or specify Cairo compiler settings, it won't make sense for
    these packages.
- During Scarb workspace resolution, all procedural macro packages are resolved and their dependencies fetched.
- Procedural macros are compiled inside the `plugins/proc_macro` subdirectory of Scarb cache.
- The procedural macro compilation is shared between Scarb projects, to ensure no recompilation on each Scarb project
  build is required.
- Procedural macros are compiled into shared libraries, with `.dylib` extension on OSX, `.so` extension on Linux
  and `.dll` on Windows.

### Scarb will build and load procedural macros on user machines

- Source distribution takes burden of building procedural macros from their authors.
  - But it requires users to have Rust toolchain installed on their machines.
    - Scarb itself does not contain any Rust source code compilation capabilities.
    - Scarb requires users to have Cargo available, in case compiling a procedural macro is required.
    - Projects that do not rely on procedural macros can be built without Rust toolchain installed.
- The procedural macros will be compiled with stable ABI layout of structs passing the FFI border. This should guarantee
  Rust ABI compatibility, regardless of Cargo toolchain version available on user machine. The `cdylib` crate type will
  be safe, and thus this should prevent runtime crashes.
- Running Rust compiler, and storing `target` directory is completely private to Scarb. Users should not influence this
  process, which should be as hermetic as possible.

### Procedural macro API in Cairo plugins

- The procedural macro has to be called from Cairo code in order to be executed during the compilation.
- The procedural macro can be triggered by one of three Cairo expressions
  - Macro call, e.g. `macro!`
  - Macro attribute, e.g. `#[macro]`
  - Macro derive, e.g. `#[derive(Macro)]`
- The API for writing procedural macros for Cairo is published for programmers, versioned separately from Scarb.
- In total, the implementation consists of three Rust crates.
  - First one, called `cairo-lang-macro`, contains the API definitions of the `TokenStream` and `ProcMacroResult`
    types used as input and output for macro implementation.
  - The second one, called `cairo-lang-macro-attributes`, contains implementation of Rust macros used for wrapping
    procedural macro entrypoint functions. These hide details of FFI communication from the procedural macro
    author.
  - The third one, called `cairo-lang-macro-stable`, contains the stable ABI versions of crates from
    the `cairo-lang-macro` crate, that can be used over the FFI communication boundary. The conversion between
    corresponding types from this two crates is implemented by the crate with API structs.
  - The first crate re-exports the contents of the second one. That's the only crate that macro authors should depend
    on.
- The procedural macro implementation is a Rust function, accepting a `TokenStream` (described in detail in
  following sections) on input and returning the expansion result as an output.
- The result struct contains the transformed `TokenStream`. Three kinds of results are possible:
  - If the `TokenStream` is the same as the input, the AST is not modified.
  - If the `TokenStream` is different from the input, the input is replaced with the generated code.
  - If the `TokenStream` is empty, the input is removed.
- Alongside the new TokenStream, a procedural macro can emit auxiliary data, encoded as an arbitrary JSON.
- The procedural macro can emit additional compiler diagnostics corresponding to the Cairo code it has been executed on.
- The procedural macro can return optional full path markers. This markers can be used to obtain the full path to marked
  items in the auxiliary data after the compilation, even though the full paths are not known when the macro is
  executed.
- The appropriate procedural macros will be executed based on the call in Cairo code by the new Cairo compiler
  internal `ProcMacroHost` plugin. This plugin will be called on each AST node and will decide if analyzed fragment
  requires code generation powered by an external plugin.
