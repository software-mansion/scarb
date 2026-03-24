# Procedural Macros

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

## Summary

Inspired by Rust's procedural macro system, Scarb procedural macros aim to bring user-defined macros support to Cairo
packages.
In general, this allows writing expressions (`macro!()`), attributes (`#[macro]`), and derive
macros (`#[derive(Macro)]`) that transform Cairo code in your package.
These transformations can be distributed as packages and loaded dynamically as package dependencies.

## Procedural macro API interface

<BigLink href="https://docs.rs/cairo-lang-macro">
Go to cairo-lang-macro documentation on docs.rs
</BigLink>

## Where to go next

- [Using procedural macros](./using)
- [Writing procedural macros](./writing)
- [Parsing token streams](./parsing)
- [Diagnostics and errors](./diagnostics)
- [Incremental compilation](./incremental)
- [Advanced topics](./advanced)
- [Design details](./appendix)
- [Examples](./examples)
- [Debugging](./debugging)
