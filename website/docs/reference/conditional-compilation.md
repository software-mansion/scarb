# Conditional Compilation

_Conditionally compiled source code_ is source code that may or may not be considered a part of the source code
depending on certain conditions.
Source code can be conditionally compiled using the [`cfg`](#the-cfg-attribute) attribute.
These conditions are based on the target environment of the compiled package and a few other miscellaneous things
further described below in detail.

_Configuration options_ are names and key-value pairs that are either set or unset.
Names are written as a single identifier such as, for example, `test`.
Key-value pairs are written as a named function argument: an identifier, `:`, and then a short string.
For example, `target: 'starknet-contract'` is a configuration option.

Keys are not unique in the set of key-value configuration options.
For example, both `opt: 'x'` and `opt: 'y'` can be set at the same time.

## Forms of conditional compilation

### The `cfg` attribute

The `cfg` attribute conditionally includes the thing it is attached to based on a configuration predicate. It is written
as `#[cfg(configuration predicate)]`. If the predicate is true, the item is rewritten to not have the `cfg` attribute on
it. If the predicate is false, the item is removed from the source code.

For example, this attribute can be used to provide different implementations of a function depending on current
Scarb [target](./targets):

```cairo
#[cfg(target: 'lib')]
fn example() -> felt252 {
    42
}

#[cfg(target: 'starknet-contract')]
fn example() -> felt252 {
    512
}
```

## Set configuration options

Which configuration options are set is determined statically during the compilation of the compilation unit of the
compiled package.
It is not possible to set a configuration option from within the source code of the package being compiled.

### `target`

Key-value option set once with the current compilation unit's [target](./targets).

Example values:

- `'lib'`
- `'starknet-contract'`

## Features

Features in Scarb provide a way to conditionally compile specific parts of the code during the build process.

### `[features]` section

A package defines a set of named features in the `[features]` section of `Scarb.toml` file. Each defined feature can list other features that should be enabled with it.

For example, a package supporting various hash functions might define features like this:

```toml
[features]
poseidon = []
pedersen = []
keccak = []
```

With these features set, conditional compilation (`cfg`) attributes can be used to selectively include code to support requested features during compile time. For instance:

```rust
// Conditionally include a module
#[cfg(feature: 'poseidon')]
mod poseidon;

// Conditionally define a function
#[cfg(feature: 'pedersen')]
fn hash_pedersen(value: felt252) -> felt252 {
  // ...
}
```

To enable specific features, use the `--features` flag followed by a comma-separated list of features. For example, to build with only the `poseidon` and `pedersen` features enabled:

```
scarb build --features poseidon,pedersen
```

Enabling all features can be done with the `--all-features` flag.

### `default` features

By default, all features are disabled unless explicitly enabled. However, this behaviour can be changed by specifying a default feature in the `[features]` section, like so:

```toml
[features]
default = ["poseidon", "pedersen"]
poseidon = []
pedersen = []
keccak = []
```

When the package is built, the default feature is enabled which in turn enables the listed features.

To disable the default feature, use the `--no-default-features` flag.

For example, in the provided scenario:

- Running `scarb build` would enable `poseidon` and `pedersen` features.
- `scarb build --features keccak` would enable `poseidon`, `pedersen`, and `keccak` features.
- `scarb build --no-default-features --features keccak` would enable only the `keccak` feature.
