# Create Output Dir

This crate provides only one function, `create_output_dir` which creates an excluded from cache directory atomically
with its parents as needed.

Under the hood, this function simply calls into [Cargo's utility crate][cargo-util], but in the future it will contain
this code
directly, reducing dependency build time.

## Changelog

All notable changes to this project are documented on the [GitHub releases] page.

## Credits

This crate is an extract from [`cargo-util`][cargo-util], developed by the Rust project contributors.

[cargo-util]: https://crates.io/crates/cargo-util

[github releases]: https://github.com/software-mansion/scarb/releases
