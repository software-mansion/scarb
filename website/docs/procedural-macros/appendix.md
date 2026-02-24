# Design Details

## Procedural macros are special Scarb packages containing Rust code

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
  and `.dll` extension on Windows.

## Scarb will build and load procedural macros on user machines

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

## Procedural macro API in Cairo plugins

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
