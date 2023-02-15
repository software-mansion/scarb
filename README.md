# Scarb

Scarb is the project management tool for the [Cairo] language.
Scarb manages your dependencies, compiles your projects and works as an extensible platform assisting in development.

## Getting started

### Installation

Binary archives for all major platforms (Linux, macOS, Windows) are published on our [GitHub releases] page.
Simply download suitable one, extract, and move the `scarb` executable to a directory reachable by your `PATH`.
In the future, an automated installer is planned to be created.

### Creating new project

```shell
$ scarb new project/directory
```

## Changelog

All notable changes to this project are documented on the [GitHub releases] page.

## Roadmap

Our goal is to release first release candidate in proximity to the Starknet v0.11 release in February.
We track project roadmap [here](https://github.com/orgs/software-mansion/projects/4/views/1).

## Credits

This product includes modified portions of code of [Cargo], developed by the Rust project contributors.

This product includes modified portions of code of [hex_solver], developed by Six Colors AB.

[Cairo]: https://www.cairo-lang.org/

[Cargo]: https://github.com/rust-lang/cargo

[github releases]: https://github.com/software-mansion/scarb/releases

[hex_solver]: https://github.com/hexpm/hex_solver
