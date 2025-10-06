# `wasm` protocol

The `wasm` oracle protocol runs a WebAssembly Component and calls one of its exported functions.
It uses WASI 0.2 (aka WASI Preview 2) as the host ABI.

This is a good fit when you want a fast, portable oracle implemented in Rust, C/C++, or any [language that can
produce](https://component-model.bytecodealliance.org/language-support.html) a WASI 0.2 WebAssembly component.

## Connection string format

```
wasm:<path-to-component.wasm>
```

The path is resolved at runtime. This file **must** be a package's [asset](../../reference/manifest.md#assets).
It can be either a binary `.wasm` or textual `.wat` file.

#### Examples

```cairo
// Call an `add` function exported by the component.
let res = oracle::invoke("wasm:mypkg-oracle.wasm", "add", (1_u64, 2_u64));

// Call `join` that takes/returns ByteArray.
let res = oracle::invoke("wasm:mypkg-oracle.wasm", "join", ("foo", "bar"));

// Call using a fully‑qualified export name.
// This is useful when there are ambiguities.
let res = oracle::invoke(
    "wasm:mypkg-oracle.wasm",
    "naked:adder/add@0.1.0/add",
    (1_i64, 2_i64)
);
```

## Execution model

- The component is instantiated once per execution and is kept alive until program terminates. Subsequent invocations to
  the same connection string reuse the same component instance and its internal state.
- Standard output: anything the component writes to stdout is forwarded to the executor's stdout (aka terminal).
- Standard error: anything the component writes to stderr is forwarded to the executor logs (visible at debug level).
- Standard input: the component inherits stdin from the executor process. Attempts to read from stdin will block the
  execution.
- Environment: the component inherits executor's environment.
- Filesystem: not implemented yet. Filesystem APIs will fail with a permissions error if used. There is no current
  working directory.
- Network: network access is enabled (WASI 0.2 sockets). DNS name lookups are allowed. Actual reachability is still
  subject to the host OS/firewall.
- Failures (including traps) never abort Cairo execution; they propagate to Cairo as an `oracle::Error` value.

> [!NOTE]
> In the future the default capabilities will become strongly limited and a permissions system will be added to allow
> enabling some of these capabilities.

## Selectors

- The selector you pass to `oracle::invoke` identifies which exported function to call.
- If the component exports multiple functions with the same short name, you must use a fully‑qualified name in the form:
  `namespace:package/world@version/func`.

## Building an oracle WASM component in Rust

WASM oracle components are standard WASI 0.2 components without any special stuff on top.
This example shows how to build a simple oracle in Rust.

::: code-group

```toml [Scarb.toml]
[package]
name = "mypkg"
version = "0.1.0"
# Make sure the compiled component is a Scarb package's asset.
assets = ["oracle/target/wasm32-wasip2/release/mypkg_oracle.wasm"] # [!code highlight]

[dependencies]
# The oracle package helps writing Cairo-side glue code.
oracle = "*" # [!code highlight]

[scripts]
# Let Scarb call Cargo to build the oracle whenever our package
# is being built or prepared for publishing.
build = "cargo build --manifest-path oracle/Cargo.toml --release --target wasm32-wasip2" # [!code highlight]
```

```cairo [src/lib.cairo]
pub fn add(left: u64, right: u64) -> oracle::Result<u64> {
    oracle::invoke(
        "wasm:mypkg_oracle.wasm",  // Resolved from assets. [!code highlight]
        "add",
        (left, right)
    )
}
```

```toml [oracle/Cargo.toml]
[package]
# Because the component is an asset, its name must be globally unique,
# so it is best to prefix it with the package name.
name = "mypkg-oracle" # [!code highlight]
version = "0.1.0"
edition = "2024"

[lib]
# This will make the Rust compiler produce a component instead of WASM CLI.
crate-type = ["cdylib"] # [!code highlight]

[dependencies]
# This crate emits special symbols that make the compiler wrap WASM
# code into WASI 0.2 component model-specific shell.
wit-bindgen = "*" # [!code highlight]
```

```wit [oracle/wit/oracle.wit]
// The exact value here does not really matter, but it is conventional
// to use the Scarb package name as the namespace.
package mypkg:oracle;

// World name also does not matter; `oracle` is a good convention.
world oracle {
    /// Returns left + right. // [!code highlight]
    export add: func(left: u64, right: u64) -> u64; // [!code highlight]
}
```

```rust [oracle/src/lib.rs]
wit_bindgen::generate!();

struct Oracle;

impl Guest for Oracle {
    fn add(left: u64, right: u64) -> u64 {
        left + right
    }
}

export!(Oracle);
```

:::

Remember to install the `wasm32-wasip2` Rust cross-compiler on your machine. You need to do this once:

```shell
rustup target add wasm32-wasip2
```

Building such a package is as simple as:

```shell
scarb build
```

## Type mapping (WIT ↔ Cairo)

The executor reads the WIT interface of the component to determine how values should be serialized and deserialized
between Cairo and WebAssembly component's ABI. This section lists all mappings the current codec supports and how values
are encoded on the wire.

| WIT                    | Cairo                  |
| ---------------------- | ---------------------- |
| `bool`                 | `bool`                 |
| `s8/s16/s32/s64`       | `i8/i16/i32/i64`       |
| `u8/u16/u32/u64`       | `u8/u16/u32/u64`       |
| `string`               | `ByteArray` (UTF-8)    |
| `char`                 | short-string `felt252` |
| `list<T>`              | `Array<T>`             |
| tuples `(T1, T2, ...)` | tuples `(T1, T2, ...)` |
| `option<T>`            | `Option<T>`            |
| `result<T, E>`         | `Result<T, E>`         |
| `result<T>`            | `Result<T, ()>`        |
| `result<_, E>`         | `Result<(), E>`        |
| `result`               | `Result<(), ()>`       |

There is no direct mapping for Cairo's `felt252`.

Unsupported WIT types (will produce an error if used):

- `float32`, `float64`
- `record`, `variant`, `enum`, `flags`
- resources (`own`/`borrow`)
- `future`, `stream`, and `error-context`
