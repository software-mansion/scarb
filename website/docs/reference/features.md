# Features

Features in Scarb provide a way to conditionally compile specific parts of the code during the build process.

## `[features]` section

In the `Scarb.toml` file, features are defined within the `[features]` section. Each feature lists other features that it enables.

For example, a package supporting various hash functions might define features like this:

```toml
[features]
poseidon = []
pedersen = []
keccak = []
```

With these features set, conditional compilation expressions (`cfg` expressions) can selectively include code to support requested features during compile time. For instance:

```rust
// Conditionally include a package
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

## `default` features

By default, all features are disabled unless explicitly enabled with the `--features` flag. However, this behavior can be changed by specifying a default feature in the `[features]` section, like so:

```toml
[features]
default = ["poseidon", "pedersen"]
poseidon = []
pedersen = []
keccak = []
```

During compilation, the compiler will enable the default feature, which in turn activates all listed features.

To disable the default feature, use the `--no-default-features` flag.

For example, in the provided scenario:
- Running `scarb build` would enable `poseidon` and `pedersen` features.
- `scarb build --features keccak` would enable `poseidon`, `pedersen`, and `keccak` features.
- `scarb build --no-default-features --features keccak` would enable only the `keccak` feature.
