# Debugging

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
