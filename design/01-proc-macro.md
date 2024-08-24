# Procedural Macros

## Summary

We are actively thinking how to allow using user-developed Cairo compiler plugins ([`MacroPlugin`]) in Scarb-powered projects. This post describes our current idea on how to approach this and outlines implementation steps. We would like to invite the community to share feedback!

This document proposes an evolution of Cairo’s compiler plugins system. The primary motivation behind this proposal is to allow dynamic loading of compiler plugins per compilation unit. This document specifies syntax for the definition of procedural macros, a high-level view of their implementation in the compiler, and outlines how they interact with the compilation process.

## Context

### Rust procedural macros

This post assumes reader knowledge on Rust's procedural macros and how they are implemented. A good walkthrough has been published some time ago on IntelliJ Rust blog:

1. https://blog.jetbrains.com/rust/2022/03/18/procedural-macros-under-the-hood-part-i/
2. https://blog.jetbrains.com/rust/2022/07/07/procedural-macros-under-the-hood-part-ii/

### System programming primitives

This post assumes reader knowledge on foreign function interface (FFI) mechanisms in modern operating systems.

### Plugins API

The Cairo compiler has a system of semantic plugins, that allow generating Cairo code with Rust code. The entry point of compiler plugins API is the [`MacroPlugin`] trait, which, via another mechanisms, boils down to allowing users to provide a callback, which:

1. Takes a reference to compiler's syntax database ([`SyntaxGroup`], i.e. compiler state until right after parsing is done).
2. Takes an [`ast::ModuleItem`].
3. Produces a struct that:
   - Can remove the [`ast::ModuleItem`].
   - Can contain Cairo code (as a virtual source file) to inject as a submodule.
   - Emits compiler diagnostics.
   - Can emit additional arbitrary data structure along generated file.

Macro plugins are executed on each node in the Cairo AST.

Custom plugins can be fed to the compiler via [`RootDatabaseBuilder::with_plugin_suite` API](https://github.com/starkware-libs/cairo/blob/8290898d602f87fb5bc3a18f28a42430cf6d3fdc/crates/cairo-lang-compiler/src/db.rs#L96). It takes a [`PluginSuite` object](https://github.com/starkware-libs/cairo/blob/8290898d602f87fb5bc3a18f28a42430cf6d3fdc/crates/cairo-lang-semantic/src/plugin.rs#L18-L25), which defines a set of objects implementing the [`MacroPlugin` trait](https://github.com/starkware-libs/cairo/blob/8290898d602f87fb5bc3a18f28a42430cf6d3fdc/crates/cairo-lang-defs/src/plugin.rs#L80).

Currently, Scarb provides a fixed set of plugins, from which the user can choose appropriate ones through Scarb manifest dependencies.
Some plugins are used as implicit dependencies, for example `test_plugin` in `test` targets.
This interface is not stable and is subject to changes in future compiler versions.

### Plugins API in combination with Scarb

Because of the use of shared reference to [`SyntaxGroup`], this API is hostile to attempts to externalize plugins into separate binaries/processes that communicate via some kind of RPC. The fact that AST structure is not stable does not help either. On the other hand, access to [`SyntaxGroup`] allows for powerful operations and is highly desired.

The only feasible way of loading plugins is to load them directly into Scarb compiler's memory. This brings another problem though: Rust does not have a stable ABI, and thus such system would have to ensure that the plugin library was compiled with **exactly** the same Rust toolchain, for example `1.68.1-x86_64-pc-windows-msvc`.

### Declaring package dependencies on plugins

Scarb allows packages to declare dependency on a plugin through the same mechanism as on standard Cairo libraries. The plugin must be defined as a Scarb package defining `cairo-plugin` target in the package manifest.

Since currently there is no mechanism to supply plugins implementation, it must be compiled into Scarb itself.
The `starknet` and `test_plugin` plugins are supplied this way.

## Guide-level explanation

This document proposes an introduction of new compiler plugins type, called procedural macros, as described below.

### Procedural macro user perspective

To use a procedural macro, a Cairo programmer will have to:

- Declare a dependency on a package, that implements the procedural macro, by adding it to the `dependencies` field in the Scarb manifest file.
- Use the procedural macro in Cairo code, by calling it, or adding an attribute or derive macro to a Cairo item.

The overall complexity of the actual procedural macro execution should be hidden from the user abstraction level.
Apart from requiring the Rust toolchain to be installed, the user will not have to perform any additional steps to use a procedural macro.
In particular, these two steps can be performed without any knowledge of Rust, or even the fact that the procedural macro is implemented in Rust.
Specifically, the following requirements will be met.

#### Procedural macro packages can be used as dependencies

- Scarb packages can simply declare dependency relationships on other packages that implement Cairo procedural macros.
- Because of semantics of Scarb package resolution, it will guarantee by itself, that only one instance of given procedural macro package exists in resolved package set.
  - In other words, Scarb will out of the box verify, that there is no simultaneous dependency on `proc-macro 1.0.0` and `proc-macro 2.0.0` or `proc-macro 1.0.1`.
- Procedural macros will end up being actual Scarb compilation unit components, though, because they will have to be treated differently from regular components, they will not be listed under `components` fields, but rather in a new one: `plugins`.

#### Procedural macro will be called from Cairo code

The procedural macro have to be called from Cairo code in order to be executed during the compilation.
In contrast to current behaviour of Cairo plugins, no longer will they be executed on each node of AST.

The procedural macro will be triggered by one of three Cairo expressions

- Macro call, e.g. `xyz!`
- Macro attribute, e.g. `#[xyz]`
- Macro derive, e.g. `#[derive(Xyz)]`

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

To implement a procedural macro, a programmer will have to:

- Create a new package, with a `Scarb.toml` manifest file and a `src/` directory besides.
- The manifest file must define a `cairo-plugin` target type.
- Write a Rust library, inside the `src/` directory, that implements the procedural macro API.
- Note, that the package does not contain any `Cargo.toml` file, as it will be generated by Scarb.
- A Rust crate exposing an API for writing procedural macros will be published for programmers. This crate will be automatically added to generated `Cargo.toml` file.
- The Rust library contained in the package will have to implement a public function responsible for code expansion.
- This function accepts a `TokenStream` as an input and returns a `ProcMacroResult` as an output, both defined in the helper library.
- The result enum will contain one of three values:
  - `ProcMacroResult::Leave` - procedural macro does not take any action
  - `ProcMacroResult::Replace({ TokenStream, AuxData })` - procedural macro has generated code that replaces input
  - `ProcMacroResult::Remove` - original code should be removed
  - `ProcMacroResult::Bail(ProcMacroDiagnostics)` - procedural macro exited with an error
- Alongside the new TokenStream, a procedural macro can emit auxiliary data, encoded as an arbitrary JSON.

A `TokenStream` definition:

- A token can be defined as a string with one of three kinds assigned to it during parsing.
- The token kinds are: identifier, punctuation symbol and literal.
- Kinds of different tokens can be represented as an enum called `TokenTree`.
- An iterator over token trees is called a `TokenStream`. A `TokenStream` is a representation of a Cairo code.
- The `TokenStream` are plain Cairo tokens that will be parsed by the procedural macro implementation itself.

Example:

```toml
[package]
name = "some-macro"
version = "0.1.0"

[cairo-plugin]
```

```cairo
use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro};

/// The entry point of procedural macro implementation.
#[inline_macro]
pub fn some_macro(token_stream: TokenStream) -> ProcMacroResult {
    // no-op
    ProcMacroResult::Leave
}
```

## Reference-level explanation

### Procedural macros will be compiled into shared libraries

- Procedural macros will be implemented in Rust.
- Procedural macros will be compiled into shared libraries, that Scarb can load at the runtime through dynamic loading mechanism of the host operating system.
- Procedural macros will be compiled to `.dylib` files on OSX, `.so` files on Linux and `.dll` on Windows.
- Scarb will use [`libloading` crate](https://docs.rs/libloading/latest/libloading/), which exposes a cross-platform interface for interactions with platform dynamic library loading primitives, to load shared libraries into memory.

### Procedural macros will be special Scarb packages containing Rust code

- A procedural macro will be a special Scarb package, which will use a new native target type: `cairo-plugin`.
- The procedural macro package will contain Rust source code, which will be shipped to users on Scarb project resolution.
- The procedural macro source code will be compiled on Scarb users system only.
- Enabling this target will mean that this package does not contain any Cairo code.
- This target will be _exclusive_:
  - It will block defining other targets for the package, not even `lib`.
  - It will also not be possible to declare dependencies, or specify Cairo compiler settings, it won't make sense for these packages.
- Scarb will automatically generate a `Cargo.toml` manifest file, based on contents of `Scarb.toml` manifest. The Cargo manifest will be used for the procedural macro compilation.
  - The `lib.crate-type` will be set `dylib`
- The Cargo manifest generation, followed by the Cargo resolution of procedural macro packages, will be handled during the Scarb workspace resolution.
- The compiled procedural macros will be cached on the user filesystem in Scarb cache directory, to ensure no recompilation on each Scarb project build is required.

**Example:**

Scarb manifest file:

```toml
[package]
name = "some-macro"
version = "0.1.0"

[cairo-plugin]
```

Automatically generated Cargo manifest file:

```toml
# Code generated by scarb fetch; DO NOT EDIT.
#
# The Cargo.toml file for Cairo procedural macros crates is entirely managed by Scarb,
# the Cairo package manager. Scarb regenerates this file whenever it processes Scarb.toml.
# The simplest way to trigger this, is to run the `scarb fetch` command.
#
# If you want to provide custom data in this file, include it in Scarb.toml under
# [tool.cargo.pkg] section. For example, to add Rust dependencies, type the following:
#
#     [tool.cargo.pkg.dependencies]
#     serde = "1"
#     serde_json = "1"

[package]
name = "ignore-plugin"
version = "0.1.0"

[lib]
crate-type = "dylib"

[dependencies]
cairo-lang-macro = "1.0.0-alpha.7"
```

### Procedural macro packages can be used as regular dependencies

See [the guide-level explanation](#Procedural-macro-packages-can-be-used-as-dependencies) for more details.

### Scarb will build and load procedural macros on user machines

- Source distribution takes burden of building procedural macros from their authors.
  - But it requires users to have Rust toolchain installed on their machines.
    - Scarb itself will not contain any Rust source code compilation capabilities.
    - Scarb will require users to have Cargo available, if it spots a procedural macro that has not been yet compiled, i.e. there is not a shared object/DLL binary in Scarb's cache.
    - Thanks to this, writing code not using dynamic procedural macros will not require having `rustc` installed.
- The procedural macros will be compiled with stable ABI layout of structs passing the FFI border. This should guarantee Rust ABI compatibility, regardless of Cargo toolchain version available on user machine. The `dylib` crate type will be safe, and thus this should prevent runtime crashes.
- Running Rust compiler, and storing `target` directory will be completely private thing to Scarb.
  - Users will not be able to influence this.
  - This will ensure procedural macro compilation will be as hermetic as possible.
- Scarb will use [`libloading`](https://crates.io/crates/libloading) for loading built procedural macros shared objects.

### Procedural macro API in Cairo plugins

- The procedural macro has to be called from Cairo code in order to be executed during the compilation. In contrast to current behaviour of Cairo plugins, no longer will they be executed on each node of AST.
- The procedural macro will be triggered by one of three Cairo expressions
  - Macro call, e.g. `xyz!`
  - Macro attribute, e.g. `#[xyz]`
  - Macro derive, e.g. `#[derive(Xyz)]`
- The API for writing procedural macros for Cairo will be published for programmers, versioned separately from Scarb. This will provide a stability for the procedural macro implementation.
- In total, the implementation will consist of three Rust crates.
  - First one, called `cairo-lang-macro`, will contain the API definitions of the `TokenStream` and `ProcMacroResult` types used as input and output for macro implementation.
  - The second one, called `cairo-lang-macro-attributes`, will contain implementation of Rust macros used for wrapping procedural macro entrypoint function. This will hide details of FFI communication from the procedural macro author.
  - The third one, called `cairo-lang-macro-stable`, will contain the stable ABI versions of crates from the `cairo-lang-macro` crate, that can be used over the FFI communication boundary. The conversion between corresponding types from this two crates will be implemented by the crate with API structs.
  - The first crate will re-export the contents of the second one. It will also be automatically added to the generated `Cargo.toml` file.
- The procedural macro implementation will be a Rust function, accepting a `TokenStream` (described in detail in following sections) on input and returning the expansion result as an output.
- The procedural macro implementation will return a `ProcMacroResult` defined in the helper library. The enum will contain one of three values:
  - `ProcMacroResult::Leave` - procedural macro does not take any action
  - `ProcMacroResult::Replace({ TokenStream, AuxData, ProcMacroDiagnostics })` - procedural macro has generated code that replaces input
  - `ProcMacroResult::Remove` - original code should be removed
- Alongside the new TokenStream, a procedural macro can emit auxiliary data, encoded as an arbitrary JSON.
- The procedural macro can emit additional compiler diagnostics corresponding to the Cairo code it has been executed on.
- The appropriate procedural macros will be executed based on the call in Cairo code by the new Cairo compiler internal `ProcMacroHost` plugin. This plugin will be called on each AST node and will decide if analyzed fragment requires code generation powered by an external plugin.

**Example:**

The procedural macro source code:

See [the guide-level explanation](#Procedural-macro-author-perspective) for an example.

The communication between Scarb and the procedural macros will have to be implemented in such a manner, that:

- Will be forward and backward compatible regardless of changes in the Cairo compiler internals (including the Cairo AST).
- Can be established over the FFI barrier.

In order to meet the specified requirements, instead of syntax tree nodes, we can rather pass plain Cairo tokens which will be parsed by the procedural macro itself.

A token can be defined as a string with one of three kinds assigned to it during parsing. The token kinds are: identifier, punctuation symbol and literal. Kinds of different tokens can be represented as an enum called TokenTree. An iterator over TokenTrees is called a TokenStream. A TokenStream is a representation of a Cairo code.

Implementation of the communication protocol will require some adjustments in the Cairo language parser, so that it can be used for TokenStreams. Namely, it should be possible to use TokenStreams as input instead of Cairo files, skipping the lexical analysis required for Cairo files (as TokenStream can already be analyzed on the compiler side).

## **Drawbacks**

- Since shared libraries are loaded into the process memory space, some errors in the execution might result in hard to debug segmentation faults.
- Procedural macros are also in fact just Rust code run on end user’s hardware. This makes them more susceptible to bad actors actions, than a standard Cairo source code. Thus, procedural macros should always be treated with special attention, and only ones from trusted sources should be used.

## **Future possibilities**

- The compiler machinery of Scarb can be extracted into a separate binary, in order to provide a higher level of isolation from the procedural macros runtime implementation. This would prevent memory corruption from killing entire Scarb execution, so Scarb would be able to recover with good error message.
- Since the communication protocol between Cairo compiler and the procedural macro will be hidden from the end user, it might be subject to optimisations along the way. Similarly to Rust proc macros, the communication protocol could be bidirectional with use of callbacks on input TokenStream, which eliminates the need to pass whole stream through FFI boundary each time.
- Procedural macro could also be executed separately from Scarb, for instance as separate processes, thanks to the separation of the procedural macro API.
- Adjust Cairo parser, so that it can be used without a SyntaxGroup db setup (which is now used to fill out the db with tokens for next steps in compilation process). The db setup is computationally expensive, while the procedural macros will not utilize it any way.

[`MacroPlugin`]: https://github.com/starkware-libs/cairo/blob/b580d12d205afa8a3212373ef1407d70eca3e7d9/crates/cairo-lang-defs/src/plugin.rs#L80
[`SyntaxGroup`]: https://github.com/starkware-libs/cairo/blob/b012f08d2442175ac0ebfe856eec60e0273b7c26/crates/cairo-lang-syntax/src/node/db.rs#L15
[`ast::ModuleItem`]: https://github.com/starkware-libs/cairo/blob/4821865770ac9e57442aef6f0ce82edc7020a4d6/crates/cairo-lang-syntax/src/node/ast.rs#L8718
