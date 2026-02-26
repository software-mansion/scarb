<script setup>
import { data as rel } from "../../github.data";
</script>

# Writing Procedural Macros

> [!WARNING]
> Procedural macros, by design, introduce a lot of overhead during the compilation.
> They may also be harder to maintain.
> Prefer the declarative inline macros written directly in Cairo, unless you have a specific reason to use procedural macros.
> Please see the [declarative macros chapter in Cairo Book](https://www.starknet.io/cairo-book/ch12-05-macros.html#declarative-inline-macros-for-general-metaprogramming) for more information.

> [!INFO]
> To use procedural macros, you need to have Rust toolchain (Cargo) installed on your machine.
> Please see [Rust installation guide](https://www.rust-lang.org/tools/install) for more information.

Scarb procedural macros are, in fact, Rust functions that take **Cairo code as input** and **return modified Cairo**
code as an output.

A procedural macro is implemented as a Rust library which defines functions that implement these transformations
(later called macro _expansions_).
This Rust code is then compiled into a shared library (shared object) and loaded into Scarb process memory during the
Cairo project compilation.
Scarb will call expansions from the loaded shared library, thus allowing you to inject custom logic to the Cairo
compilation process.

## Procedural macro author perspective

To implement a procedural macro, a programmer has to:

- Create a new package, with a `Scarb.toml` manifest file, `Cargo.toml` manifest file and a `src/` directory besides.
- The Scarb manifest file must define a `cairo-plugin` target type.
- The Cargo manifest file must define a `crate-type = ["cdylib"]` on `[lib]` target.
- Write a Rust library, inside the `src/` directory that implements the procedural macro API.
- A Rust crate exposing an API for writing procedural macros is published for programmers under the
  name `cairo-lang-macro`. This crate must be added to the `Cargo.toml` file.
- The Rust library contained in the package has to implement a function responsible for code expansion.
- This function accepts a `TokenStream` as an input and returns a `ProcMacroResult` as an output, both defined in the
  helper library.
- The result struct contains the transformed `TokenStream`. Three kinds of results are possible:
  - If the `TokenStream` is the same as the input, the AST is not modified.
  - If the `TokenStream` is different from the input, the input is replaced with the generated code.
  - If the `TokenStream` is empty, the input is removed.
- Alongside the new TokenStream, a procedural macro can emit compiler diagnostics, auxiliary data and full path
  identifiers, described in detail in advanced macro usage section.

We define `TokenStream` as some encapsulation of code represented in plain Cairo.

## Creating procedural macros with helpers

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

## Example projects

See the [examples](./examples) for working end-to-end macro implementations.

### Minimal example: a macro that removes code

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

Maybe it's not the most productive code you wrote, but the function has been removed during the compilation.
