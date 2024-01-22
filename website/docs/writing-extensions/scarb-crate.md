# Using Scarb as a library <Badge type="warning" text="deprecated" />

> [!WARNING]
> Scarb crate is scheduled for deprecation in unspecified future.
> It is advised not to use it in new projects.
>
> The combination of calling scarb command with the `--json` flag, and the `scarb metadata` command should cover all use
> cases for communicating with Scarb from outside world.

> [!WARNING]
> Scarb is not being published to crates.io anymore.
> Use Scarb via Git reference in your `Cargo.toml`.

Scarb is a [Rust](https://rust-lang.org) crate which can be used as a regular library in Rust applications.
We publish each release of Scarb to [crates.io](https://crates.io), the official package registry for Rust.

This crate serves as an API for writing custom extensions for Scarb.
It is not recommended to link to Scarb in regular unrelated applications, for this it is preferred to interact with
Scarb binary.
