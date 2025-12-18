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

### Set configuration options

Which configuration options are set is determined statically during the compilation of the compilation unit of the
compiled package.
It is not possible to set a configuration option from within the source code of the package being compiled.

#### `target`

Key-value option set once with the current compilation unit's [target](./targets).

Example values:

- `'lib'`
- `'starknet-contract'`
- `'test'`

### Conditional compilation of tests

You can use the `cfg` attribute to control which parts of your project are compiled when compiling tests.

Two configuration options will be helpful for this:

- `#[cfg(test)]` - items under this attribute will be compiled in test builds (i.e. `scarb build --test`), but only in the package that is tested, not in its dependencies.
- `#[cfg(target: 'test')]` - items under this attribute will be compiled in test build, both in the tested package, in its dependents and dependencies.

All tests of your package should be under `#[cfg(test)]`, while the `#[cfg(target: 'test')]` might be helpful, if you write libraries for testing other Cairo code.
Be careful when using the `#[cfg(target: 'test')]` attribute!
Exposing some Cairo code (like some functions tagged with `#[test]` attribute) under this attribute may cause the compilation of a dependant of the library to fail!

## Features

Features in Scarb provide a way to conditionally compile specific parts of the code during the build process.

### `[features]` section

A package defines a set of named features in the `[features]` section of `Scarb.toml` file. Each defined feature can
list other features that should be enabled with it.

For example, a package supporting various hash functions might define features like this:

```toml
[features]
poseidon = []
pedersen = []
keccak = []
```

With these features set, conditional compilation (`cfg`) attributes can be used to selectively include code to support
requested features during compile time. For instance:

```cairo
// Conditionally include a module
#[cfg(feature: 'poseidon')]
mod poseidon;

// Conditionally define a function
#[cfg(feature: 'pedersen')]
fn hash_pedersen(value: felt252) -> felt252 {
  // ...
}
```

To enable specific features, use the `--features` flag followed by a comma-separated list of features. For example, to
build with only the `poseidon` and `pedersen` features enabled:

```
scarb build --features poseidon,pedersen
```

Enabling all features can be done with the `--all-features` flag.

### `default` features

By default, all features are disabled unless explicitly enabled. However, this behaviour can be changed by specifying a
default feature in the `[features]` section, like so:

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

## Dependency features

Features of dependencies can be enabled within the dependency declaration. The features key indicates which features to enable:

```toml
[dependencies]
hashes = { path = '../hashes', features = ["poseidon", "pedersen"] }
```

The default features can be disabled using default-features = false:

```toml
[dependencies]
hashes = { path = '../hashes', features = ["poseidon"], default-features = false }
```

> [!WARNING]
> Note: This may not ensure the default features are disabled. If another dependency includes `hashes` without specifying `default-features = false`, then the default features will be enabled. See feature unification below for more details.

Features of dependencies can also be enabled in the `[features]` table. The syntax is `package-name/feature-name`. For example:

```toml
[dependencies]
hashes = { version = "0.1.0", default-features = false }

[features]
poseidon = ["hashes/poseidon"]
```

## Feature unification

Features are unique to the package that defines them.
Enabling a feature on a package does not enable a feature of the same name on other packages.

When a dependency is used by multiple packages, Scarb will use the union of all features enabled on that dependency
when building it.

For example, assume a package called `hashes` that defines the features `poseidon`, `pedersen`, and `keccak`.

```toml
[features]
poseidon = []
pedersen = []
keccak = []
```

If your package depends on a package `foo` which enables the `poseidon` and `pedersen` features of `hashes`, and another
dependency `bar` which enables the `pedersen` and `keccak` features of `hashes`, then `hashes` will be built with all
three of those features enabled.

> [!WARNING]
> A consequence of this is that **features should be additive**. That is, enabling a feature should not disable functionality,
> and it should usually be **safe to enable any combination of features**.
> A feature should not introduce a SemVer-incompatible change.

## SemVer compatibility

Enabling a feature should not introduce a SemVer-incompatible change. For example, the feature shouldn’t change an
existing API in a way that could break existing uses.

Care should be taken when adding and removing feature definitions and optional dependencies, as these can sometimes be
backwards-incompatible changes.
In short, follow these rules:

- The following is usually safe to do in a minor release:
  - Add a new feature or optional dependency.
  - Change the features used on a dependency.
- The following should usually not be done in a minor release:
  - Remove a feature or optional dependency.
  - Moving existing public code behind a feature.
  - Remove a feature from a feature list.
