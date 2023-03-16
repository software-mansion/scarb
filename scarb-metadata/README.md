# `scarb-metadata`

This crate provides structured access to the output of `scarb metadata` command.
[Scarb](https://docs.swmansion.com/scarb) is a build toolchain and package manager for
the [Cairo language](https://www.cairo-lang.org/).
See the [Scarb documentation](https://docs.swmansion.com/scarb/docs) for details on
Scarb itself.

With the `command` feature (enabled by default), it also exposes an ergonomic interface to collect metadata from Scarb.

## Credits

This crate has been inspired by, and includes relevant portions of, its Cargo
counterpart [`cargo_metadata`](https://crates.io/crates/cargo_metadata) developed by Oli Scherer.
