# Scarb metadata command

You can use the `scarb metadata` command to get information about package structure and dependencies.
The format is stable and versioned.
When calling `scarb metadata`, you are required to pass `--format-version` flag explicitly.

See `scarb metadata --help` for more information about accepted arguments.

## Reading metadata from Rust

If you are using Rust, the `scarb-metadata` crate can be used to invoke the `scarb metadata` command appropriately and parse its output.

<BigLink href="https://crates.io/crates/scarb-metadata">
  Go to scarb-metadata on crates.io
</BigLink>

<BigLink href="https://docs.rs/scarb-metadata">
  Go to scarb-metadata documentation on docs.rs
</BigLink>
