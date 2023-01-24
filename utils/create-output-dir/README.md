# Create Output Dir

This crate provides only one function, `create_output_dir` which creates an excluded from cache directory atomically
with its parents as needed.

The source code of this crate has been almost verbatim copy-pasted from
[`cargo_util::paths::create_dir_all_excluded_from_backups_atomic`][cargo-util-fn].

## Changelog

All notable changes to this project are documented on the [GitHub releases] page.

## Credits

This crate is an extract from [`cargo-util`][cargo-util], developed by the Rust project contributors.

[cargo-util]: https://crates.io/crates/cargo-util

[cargo-util-fn]: https://docs.rs/cargo-util/latest/cargo_util/paths/fn.create_dir_all_excluded_from_backups_atomic.html

[github releases]: https://github.com/software-mansion/scarb/releases
