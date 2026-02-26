<script setup>
import { data as rel } from "../../github.data";
</script>

# Parsing Token Streams

> [!WARNING]
> Procedural macros, by design, introduce a lot of overhead during the compilation.
> They may also be harder to maintain.
> Prefer the declarative inline macros written directly in Cairo, unless you have a specific reason to use procedural macros.
> Please see the [declarative macros chapter in Cairo Book](https://www.starknet.io/cairo-book/ch12-05-macros.html#declarative-inline-macros-for-general-metaprogramming) for more information.

> [!INFO]
> To use procedural macros, you need to have Rust toolchain (Cargo) installed on your machine.
> Please see [Rust installation guide](https://www.rust-lang.org/tools/install) for more information.

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
