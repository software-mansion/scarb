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

_**Also:** There is `scarb init` which runs in current directory instead of creating new one._

### Compiling

```shell
$ scarb build
```

Built artifacts will be written to `target/release` directory.

_**Also:** `scarb clean` cleans `target` directory._

#### Building CASM

Add following to `Scarb.toml`:

```toml
[lib]
casm = true
```

_**Also:** Adding `sierra = false` will stop building Sierra code._

#### Building StarkNet contracts

Add following to `Scarb.toml`:

```toml
[[target.starknet-contract]]
```

**Note:** Ensure there is no `[lib]` section, they are conflicting
until https://github.com/software-mansion/scarb/issues/63 will be done.

### Adding dependencies

**Note:** This is identical to Cargo.

#### In manifest

```toml
[dependencies]
quaireaux = { path = "path/to/quaireaux" }
quaireaux = { git = "https://github.com/keep-starknet-strange/quaireaux.git" }
```

_**Also:** You can add `version` field to specify package version, but this will do more harm than good currently, Scarb
lacks proper version solution algorithm yet._

_**Also:** You can add `branch`, `tag` and `rev` fields to Git dependencies._

_**Also:** You can use `ssh://` URLs, Scarb uses local `git` installation for all network operations._

#### Via `scarb add`

```shell
$ scarb add quaireaux --path path/to/quaireaux
$ scarb add quaireaux --git https://github.com/keep-starknet-strange/quaireaux.git
```

_**Also:** You can specify package version like this: `quaireaux@0.1.0`, but see remarks in previous section._

_**Also:** `--git` supports `--branch`, `--tag` and `--rev` arguments._

_**Also:** `scarb rm` removes a dependency._

### Formatting

```shell
# Format Cairo code:
$ scarb fmt

# Check formatting in CI:
$ scarb fmt -c
```

## Changelog and roadmap

All notable changes to this project are documented on the [GitHub releases] page.

We track project roadmap [here](https://github.com/orgs/software-mansion/projects/4/views/1).

## Credits

This product includes modified portions of code of [Cargo], developed by the Rust project contributors.

This product includes modified portions of code of [hex_solver], developed by Six Colors AB.

[Cairo]: https://www.cairo-lang.org/

[Cargo]: https://github.com/rust-lang/cargo

[github releases]: https://github.com/software-mansion/scarb/releases

[hex_solver]: https://github.com/hexpm/hex_solver
