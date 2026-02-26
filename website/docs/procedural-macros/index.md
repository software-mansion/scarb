# Procedural Macros

> [!WARNING]
> Procedural macros, by design, introduce a lot of overhead during the compilation.
> They may also be harder to maintain.
> Prefer the declarative inline macros written directly in Cairo, unless you have a specific reason to use procedural macros.
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
