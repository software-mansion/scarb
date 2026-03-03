# Using Procedural Macros

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

## Procedural macro user perspective

To use a procedural macro, a Cairo programmer needs to:

- Declare a dependency on a package, that implements the procedural macro, by adding it to the `dependencies` section in
  the Scarb manifest file.
- Use the procedural macro in Cairo code, by calling it, or adding an attribute or derive macro to a Cairo item.

Since Scarb procedural macros are, in fact, Rust functions that are usually distributed as source code and compiled into
shared libraries (see [writing a procedural macro](./writing) for more details) on the user side,
users are **required to have Rust toolchain installed** on their machine.
This limitation can be omitted by distributing procedural macros as precompiled shared libraries, see
[prebuilt procedural macros](./advanced#prebuilt-procedural-macros) for more details.

Apart from this requirement, the user does not have to perform any additional steps to use a procedural macro.
In particular, these two steps can be performed without any knowledge of Rust, or even the fact that the procedural
macro is implemented in Rust.

### Procedural macro must be called from Cairo code

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
// Derive macro `ToValue` that's imported from `tracing-macro` dependency.
#[derive(ToValue)]
struct Input {
    value: felt252,
}

// Attribute macro `instrument` that's imported from `tracing-macro` dependency.
#[instrument]
fn main() -> felt252 {
    let a = Input { value: 1 };
    let b = Input { value: 2 };
    // Inline macro `add!` that's imported from `add-macro` dependency.
    add!(a.to_value(), b.to_value());
}
```

### Procedural macro packages can be used as dependencies

- Scarb packages can simply declare dependency relationships on other packages that implement Cairo procedural macros.
- The semantics of Scarb package resolution guarantee that only one instance of a given
  procedural macro package exists in the resolved package set.
  - In other words, Scarb will out of the box verify that there is no simultaneous dependency on `proc-macro 1.0.0`
    and `proc-macro 2.0.0` or `proc-macro 1.0.1`.
- Procedural macros will end up being actual Scarb compilation unit components, though, because they will have to be
  treated differently from regular components, they will not be listed under `components` fields, but rather in a new
  one: `plugins`.
