# Debugging

> [!WARNING]
> Procedural macros, by design, introduce a lot of overhead during the compilation.
> They may also be harder to maintain.
> Prefer the declarative inline macros written directly in Cairo, unless you have a specific reason to use procedural macros.
> There are several reasons for this:
>
> - **Compilation overhead**: procedural macros are Rust crates compiled into shared libraries, adding a full Rust compilation step (via Cargo) on top of the Cairo build and significantly increasing build times.
> - **Rust toolchain dependency**: anyone using your macro must have the Rust toolchain (Cargo) installed on their machine (unless the macro is distributed as a precompiled shared library, which is not always the case).
> - **Harder to debug**: errors in macro expansion surface as confusing Cairo compiler diagnostics, making them difficult to diagnose.
> - **Higher maintenance burden**: they require knowledge of both Rust and Cairo, and the FFI boundary between them adds complexity.
>
> Please see the [declarative macros chapter in Cairo Book](https://www.starknet.io/cairo-book/ch12-05-macros.html#declarative-inline-macros-for-general-metaprogramming) for more information.

> [!INFO]
> To use procedural macros, you need to have Rust toolchain (Cargo) installed on your machine.
> Please see [Rust installation guide](https://www.rust-lang.org/tools/install) for more information.

## Inspecting the Syntax Tree

Debugging procedural macros can be challenging. A practical approach is to inspect the syntax tree of the input using `cairo_lang_parser::printer::print_tree`. This function generates a readable representation of the parsed syntax structure, which you can print and examine to understand the token stream you're working with.

```rust
use cairo_lang_parser::{printer::print_tree, utils::SimpleParserDatabase};

fn my_macro(_args: TokenStream, body: TokenStream) -> ProcMacroResult {
    let db = SimpleParserDatabase::default();
    let (node, _diagnostics) = db.parse_token_stream(&body);

    // This section is used only for macro debugging purposes.
    // This way, we can see the exact syntax structure of the item we want to modify.
    let node_tree = print_tree(&db, &node, false, false);
    println!("node tree: \n{}", node_tree);
    ...
}
```

If you are using the [Cairo Visual Studio Code extension](https://marketplace.visualstudio.com/items?itemName=starkware.cairo1), there's a special command available `View syntax tree of the current file content`, which displays the syntax tree of the current Cairo file in a separate tab.
