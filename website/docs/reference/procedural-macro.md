# Procedural Macros

> [!WARNING]
> To use procedural macros, you need to have Rust toolchain (Cargo) installed on your machine.
> Please see [Rust installation guide](https://www.rust-lang.org/tools/install) for more information.

## Summary

Inspired by Rust's procedural macro system, Scarb procedural macros aim is to bring user-defined macros support to Cairo
packages.
In general, this allows writing expressions (`macro!()`), attributes (`#[macro]`), and derive
macros (`#[derive(Macro)]`) that transform Cairo code in your package.
This transformations can be loaded dynamically per compilation unit as dependencies.

### Procedural macro API interface

<BigLink href="https://docs.rs/cairo-lang-macro">
Go to cairo-lang-macro documentation on docs.rs
</BigLink>

## Guide-level explanation

### Procedural macro user perspective

To use a procedural macro, a Cairo programmer will have to:

- Declare a dependency on a package, that implements the procedural macro, by adding it to the `dependencies` section in
  the Scarb manifest file.
- Use the procedural macro in Cairo code, by calling it, or adding an attribute or derive macro to a Cairo item.

Since Scarb procedural macros are in fact Rust functions, that are distributed as source code and compiled into shared
libraries, users are required to have Rust toolchain installed on their machine.
Apart from this requirement, the user will not have to perform any additional steps to use a procedural macro.
In particular, these two steps can be performed without any knowledge of Rust, or even the fact that the procedural
macro is implemented in Rust.

Specifically, following points are true:

#### Procedural macro packages can be used as dependencies

- Scarb packages can simply declare dependency relationships on other packages that implement Cairo procedural macros.
- Because of semantics of Scarb package resolution, it will guarantee by itself, that only one instance of given
  procedural macro package exists in resolved package set.
  - In other words, Scarb will out of the box verify, that there is no simultaneous dependency on `proc-macro 1.0.0`
    and `proc-macro 2.0.0` or `proc-macro 1.0.1`.
- Procedural macros will end up being actual Scarb compilation unit components, though, because they will have to be
  treated differently from regular components, they will not be listed under `components` fields, but rather in a new
  one: `plugins`.

#### Procedural macro will be called from Cairo code

The procedural macro have to be called from Cairo code in order to be executed during the compilation.
In contrast to current behaviour of Cairo plugins, no longer will they be executed on each node of AST.

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
use add_macro::add;
use tracing_macro::instrument;
use to_value_macro::ToValue;

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

### Procedural macro author perspective

Scarb procedural macros are in fact Rust functions, that take Cairo code as input and return modified Cairo code as an
output.

To implement a procedural macro, a programmer have to:

- Create a new package, with a `Scarb.toml` manifest file, `Cargo.toml` manifest file and a `src/` directory besides.
- The Scarb manifest file must define a `cairo-plugin` target type.
- The Cargo manifest file must define a `crate-type = ["cdylib"]` on `[lib]` target.
- Write a Rust library, inside the `src/` directory that implements the procedural macro API.
- A Rust crate exposing an API for writing procedural macros is published for programmers under the
  name `cairo-lang-macro`. This crate must be added to the `Cargo.toml` file.
- The Rust library contained in the package have to implement a functions responsible for code expansion.
- This function accepts a `TokenStream` as an input and returns a `ProcMacroResult` as an output, both defined in the
  helper library.
- The result struct contains the transformed `TokenStream`. Three kinds of results are possible:
  - If the `TokenStream` is the same as the input, the AST is not modified.
  - If the `TokenStream` is different from the input, the input is replaced with the generated code.
  - If the `TokenStream` is empty, the input is removed.
- Alongside the new TokenStream, a procedural macro can emit compiler diagnostics, auxiliary data and full path
  identifiers, described in detail in advanced macro usage section.

We define token stream as some encapsulation of code represented in plain Cairo.
Token stream can be converted into a String with `to_string()` method.

### Creating procedural macros with helpers

To simplify the process of writing procedural macros, a set of helper macros is provided in the `cairo-lang-macro`.
This helpers are implemented as Rust procedural macros that hide the details of FFI communication from Scarb procedural
macro author.
These three macro helpers are:

1. #[`inline_macro`] - Implements an expression macro. Should be used on function that accepts single token stream.
2. #[`attribute_macro`] - Implements an attribute macro. Should be used on function that accepts two token streams -
   first for the attribute arguments (`#[macro(arguments)]`) and second for the item the attribute is applied to.
3. #[`derive_macro`] - Implements a derive macro. Should be used on function that accepts single token stream, the item
   the derive is applied to. Note that derives cannot replace the original item, but rather add new items to the module.

Please review the [`cairo-lang-macro` documentation](https://docs.rs/cairo-lang-macro) for more information.

### Parsing token streams

To parse Cairo code, you can use the `cairo-lang-parser` crate, defined in the Cairo compiler repository and available
on crates.io.
The parser implemented there provides two helpful methods `parse_virtual` and `parse_virtual_with_diagnostics`, which
accept token streams.

Example:

```rust
use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro};
use cairo_lang_parser::utils::SimpleParserDatabase;

#[inline_macro]
pub fn some(token_stream: TokenStream) -> ProcMacroResult {
    let db = SimpleParserDatabase::default();
    // To obtain parser diagnostics alongside parsed node.
    let (parsed_node, diagnostics) = db.parse_virtual_with_diagnostics(token_stream);
    // To obtain parsed node only, returning any diagnostics as an error.
    let parsed_node = db.parse_virtual(token_stream).unwrap();
    (...)
}
```

### Procedural macro example:

```toml
# Scarb.toml
[package]
name = "some_macro"
version = "0.1.0"

[cairo-plugin]
```

```toml
# Cargo.toml
[package]
name = "some_macro"
version = "0.1.0"
edition = "2021"
publish = false

[lib]
crate-type = ["cdylib"]

[dependencies]
cairo-lang-macro = "0.1.0"
```

```rust
// src/lib.rs
use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro};

/// The entry point of procedural macro implementation.
#[inline_macro]
pub fn some(token_stream: TokenStream) -> ProcMacroResult {
    // no-op
    ProcMacroResult::new(token_stream)
}
```

## Reference-level explanation

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

### Procedural macro packages can be used as regular dependencies

See [the guide-level explanation](#Procedural-macro-packages-can-be-used-as-dependencies) for more details.

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

### Advanced macro usage

#### Token stream metadata

As defined before, token stream is an encapsulation of Cairo code, that can be converted into a string.
Additionally, token stream passed to the procedural macro contains metadata about the fragment of Code received.
This metadata is represented by the `TokenStreamMetadata` struct, which contains the following fields:

- `original_file_path` - The path to the file in users filesystem, from which the Cairo code was read.
- `file_id` - An identifier assigned to the file by Scarb. This identifier is guaranteed to uniquely identify file
  across all files in the Scarb project.

All fields in metadata struct are optional, but will be present in the token stream you receive from Scarb for
expansion.

This metadata can be obtained by calling `.metadata()` method on `TokenStream` struct.

#### Diagnostics

Procedural macros can emit compiler diagnostics, which will be displayed as warnings / errors to the user during the
compilation.
Diagnostics should be used to inform users about mistakes they made in their Cairo code, ideally suggesting a fix.

Exemplary diagnostic reported to user:

```shell
error: Inline macro `some` failed.
--> [..]lib.cairo:2:14
    let _x = some!();
             ^*****^
```

Diagnostics emitted within procedural macro will be displayed in the terminal, and the caret will be pointing to the
place in users Cairo code, where the procedural macro is called.

To emit diagnostics, the `with_diagnostics` method on `ProcMacroResult` struct can be used.
Diagnostics can be emitted with two levels of severity: `error` and `warning`.

```cairo
use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, Diagnostic};

#[attribute_macro]
pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
  let diag = Diagnostic::error("Some error from macro.");
  ProcMacroResult::new(token_stream)
    .with_diagnostics(diag.into())
}
```

#### Auxiliary data

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
