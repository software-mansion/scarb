# Cheat sheet

Get started with Scarb by following a cheatsheet of the most important commands.

## Creating new project

```shell
scarb new project/directory
```

::: info
There is `scarb init` which runs in current directory instead of creating new one.
:::

## Compiling

```shell
scarb build
```

Built Sierra code of this package will be written to `target/dev` directory.

::: info
`scarb clean` cleans `target` directory.
:::

### Building CASM

Add following to `Scarb.toml`:

```toml
[lib]
casm = true
```

Adding following line to `[lib]` section will stop building Sierra code:

```toml
sierra = false
```

### Building StarkNet contracts

Add following to `Scarb.toml`:

```toml
[dependencies]
starknet = "2.1.0"

[[target.starknet-contract]]
```

## Adding dependencies

### In manifest

Add dependency hosted on a Git repository:

```toml
[dependencies]
alexandria = { git = "https://github.com/keep-starknet-strange/alexandria.git" }
```

Add dependency located in local path:

```toml
[dependencies]
alexandria = { path = "../path-to-alexandria-checkout/alexandria" }
```

::: info
You can add `version` field to specify package version requirement.
:::

::: info
You can add `branch`, `tag` and `rev` fields to Git dependencies.
:::

::: info
You can use `ssh://` URLs, Scarb uses local `git` installation for all network operations.
:::

### Via `scarb add`

Add dependency hosted on a Git repository:

```shell
scarb add alexandria --git https://github.com/keep-starknet-strange/alexandria.git
```

Add dependency located in local path:

```shell
scarb add alexandria --path ../path-to-alexandria-checkout/alexandria
```

::: info
You can specify package version like this: `alexandria@0.1.0`, but see remarks in previous section.
:::

::: info
`--git` supports `--branch`, `--tag` and `--rev` arguments.
:::

::: info
`scarb rm` removes a dependency.
:::

## Formatting

Format Cairo code:

```shell
scarb fmt
```

Check formatting in CI:

```shell
scarb fmt -c
```
