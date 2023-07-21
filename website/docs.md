# Introduction

Scarb is the [Cairo](https://cairo-lang.org) package manager.
Scarb downloads your Cairo package's dependencies, compiles your projects (either pure Cairo or StarkNet contracts),
and works as an entry point for other tooling to work with your code, such
as [Starknet Foundry](https://foundry-rs.github.io/starknet-foundry) or IDEs.

Scarb is heavily inspired by [Cargo](https://doc.rust-lang.org/cargo/).
The goal is to make programmers used to writing Rust feel at home.

## Installation

Check out the [download](/download) page for installation instruction and release archives.

## Command line help

Scarb is designed to be discoverable straight from the terminal.
If you want to get the information about available commands and flags, you can always use:

```shell
scarb -h
```

Or, for even more details:

```shell
scarb --help
```

If you want to get the detailed information about a certain command and available flags, you can always use:

```shell
scarb COMMAND --help
```

For example:

```shell
scarb build --help
```
