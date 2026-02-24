# Advanced Topics

## Controlling Rust version

Scarb will compile your procedural macro with Cargo available on your (macro users) system.
The macro will be compiled from the directory where the macro is defined.
This means that if you use some macro as a dependency, it will be compiled from a cache directory where it's been
downloaded by Scarb.
If you manage your Rust version through Rustup, it will use the same directory to determine which Rust version to use.
This implies that if you define some specific Rust toolchain version override, for the package that depends on some macro,
the macro can be compiled with a different toolchain version, as the Cairo package is not part of the Rust compilation.

To control the Rust toolchain version, you can globally override the toolchain version by setting the default version
with rustup or temporarily override it with `RUSTUP_TOOLCHAIN` environment variable.
See [rustup documentation](https://rust-lang.github.io/rustup/overrides.html) for more details.

You can also override the Cargo binary that will be called by Scarb, by setting `CARGO` environment variable.

## Prebuilt procedural macros

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
For publishing, [the `include` field](/docs/reference/manifest#include) of the package manifest may be useful, as it can be used
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

## Token stream metadata

As defined before, token stream is an encapsulation of Cairo code, that can be converted into a string.
Additionally, token stream passed to the procedural macro contains metadata about the fragment of Code received.
This metadata is represented by the `TokenStreamMetadata` struct, which contains the following fields:

- `original_file_path` - The path to the file in users filesystem, from which the Cairo code was read.
- `file_id` - An identifier assigned to the file by Scarb. This identifier is guaranteed to uniquely identify file
  across all files in the Scarb project.

All fields in metadata struct are optional, but will be present in the token stream you receive from Scarb for
expansion.

This metadata can be obtained by calling `.metadata()` method on `TokenStream` struct.

## Auxiliary data

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
