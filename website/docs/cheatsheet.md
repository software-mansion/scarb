<script setup>
import { data as rel } from "../github.data";
</script>

# Cheat sheet

Get started with Scarb by following a cheatsheet of the most important commands.

## Creating new project

```shell
scarb new project/directory
```

> [!TIP]
> There is `scarb init` which runs in current directory instead of creating new one.

## Compiling

```shell
scarb build
```

Built Sierra code of this package will be written to `target/dev` directory.

<!-- prettier-ignore -->
> [!TIP]
> `scarb clean` cleans `target` directory.

<!-- prettier-ignore -->
> [!TIP]
> `scarb check` checks for errors without compiling the packages.

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

```toml-vue
[dependencies]
starknet = "{{ rel.stable.starknetPackageVersionReq }}"

[[target.starknet-contract]]
```

## Adding dependencies

### In manifest

Add dependency hosted on the official [scarbs.xyz](https://scarbs.xyz) registry:

```toml
[dependencies]
alexandria_math = "0.1.0"
```

Add dependency hosted on a Git repository:

```toml
[dependencies]
alexandria_math = { git = "https://github.com/keep-starknet-strange/alexandria.git" }
```

Add dependency located in local path:

```toml
[dependencies]
alexandria_math = { path = "../path-to-alexandria-checkout/alexandria" }
```

> [!TIP]
> You can add `version` field to specify package version requirement.

> [!TIP]
> You can add `branch`, `tag` and `rev` fields to Git dependencies.

> [!TIP]
> You can use `ssh://` URLs, Scarb uses local `git` installation for all network operations.

> [!TIP]
> You can add development dependencies in `[dev-dependencies]` section.

### Via `scarb add`

Add dependency from the official [scarbs.xyz](https://scarbs.xyz) registry:

```shell
scarb add alexandria_math@0.1.0
```

Add dependency hosted on a Git repository:

```shell
scarb add alexandria_math --git https://github.com/keep-starknet-strange/alexandria.git
```

Add dependency located in local path:

```shell
scarb add alexandria_math --path ../path-to-alexandria-checkout/alexandria
```

> [!TIP]
> You can specify package version like this: `alexandria_math@0.1.0`, but see remarks in previous section.

<!-- prettier-ignore -->
> [!TIP]
> `--git` supports `--branch`, `--tag` and `--rev` arguments.

<!-- prettier-ignore -->
> [!TIP]
> `scarb rm` removes a dependency.

<!-- prettier-ignore -->
> [!TIP]
> `--dev` flag adds/removes a development dependency.

## Formatting

Format Cairo code:

```shell
scarb fmt
```

Check formatting in CI:

```shell
scarb fmt -c
```

## Updating locked dependencies

```shell
scarb update
```
