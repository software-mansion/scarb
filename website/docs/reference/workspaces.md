# Workspaces

A workspace is a collection of one or more packages, called workspace members, that are managed together.

The key points of workspaces are:

- Common commands can run across all workspace members, like `scarb build --workspace`.
- All packages share a common output directory, which defaults to a directory named target in the _workspace root_.
- Sharing package metadata, like with _workspace.package_.
- The `[profile.*]` section in the manifest file is only recognized in the root manifest, and ignored in member
  manifests.

In a manifest file, the `[workspace]` table supports the following sections:

## `[workspace]`

To create a workspace, you add the `[workspace]` table to `Scarb.toml`:

```toml
[workspace]
# ...
```

At minimum, a workspace has to have a member, either with a root package or as a virtual manifest.

### Root package

If the `[workspace]` section is added to `Scarb.toml` that already defines a `[package]`,
the package is the root package of the workspace.
The workspace root is the directory where the workspace’s `Scarb.toml` is located.

```toml
[workspace]

[package]
name = "hello_world" # the name of the package
version = "0.1.0"    # the current version, obeying semver
authors = ["Alice <a@example.com>", "Bob <b@example.com>"]
```

### Virtual workspace

Alternatively, a `Scarb.toml` file can be created with a `[workspace]` section but without a `[package]` section.
This is called a virtual manifest.
This is typically useful when there isn’t a “primary” package, or you want to keep all the packages organized
in separate directories.

::: code-group

```toml [Scarb.toml]
[workspace]
members = ["hello_world"]
```

```toml [hello_world/Scarb.toml]
[package]
name = "hello_world" # the name of the package
version = "0.1.0"    # the current version, obeying semver
authors = ["Alice <a@example.com>", "Bob <b@example.com>"]
```

:::

### `[members]`

The _members_ fields define which packages are members of the workspace.

Additional members can be listed with the members key, which should be an array of strings containing directories with
`Scarb.toml` files.

The members list also supports [globs](https://docs.rs/glob/0.3.0/glob/struct.Pattern.html) to match multiple paths,
using typical filename glob patterns like `*` and `?`.

When inside a subdirectory within the workspace, Scarb will automatically search the parent directories for
a `Scarb.toml` file with a `[workspace]` definition to determine which workspace to use.

### Package selection

In a workspace, package-related Scarb commands like `scarb build` can use the `-p / --package` or `--workspace`
command-line flags to determine which packages to operate on.
If neither of those flags are specified, Scarb will use the package in the current working directory.
If the current directory is a virtual workspace, it will apply to all members
(as if `--workspace` were specified on the command-line).

### `[package]`

The `workspace.package` table is where you define keys that can be inherited by members of a workspace.
These keys can be inherited by defining them in the member package with `{key}.workspace = true`.

Keys that are supported:

- `version`
- `authors`
- `description`
- `documentation`
- `homepage`
- `keywords`
- `license`
- `license-file`
- `readme`
- `repository`
- `cairo-version`

(See [manifest](./manifest) for more information on the meaning of inheritable keys.)

Example:

::: code-group

```toml [Scarb.toml]
[workspace]
members = ["bar"]

[workspace.package]
version = "1.2.3"
authors = ["Nice Folks"]
description = "A short description of my package"
documentation = "https://example.com/bar"
```

```toml [bar/Scarb.toml]
[package]
name = "bar"
version.workspace = true
authors.workspace = true
description.workspace = true
documentation.workspace = true
```

:::

### `[dependencies]`

The `workspace.dependencies` table is where you define dependencies to be inherited by members of a workspace.

Specifying a workspace dependency is similar to package dependencies,
except you can then inherit the workspace dependency as a package dependency

Example:

::: code-group

```toml [Scarb.toml]
[workspace]
members = ["foo", "bar"]

[workspace.dependencies]
alexandria_math = { git = "https://github.com/keep-starknet-strange/alexandria.git" }
openzeppelin = { git = "https://github.com/OpenZeppelin/cairo-contracts.git", branch = "cairo-2" }
```

```toml [foo/Scarb.toml]
[package]
name = "foo"
version = "0.2.0"

[dependencies]
alexandria_math.workspace = true
```

```toml [bar/Scarb.toml]
[package]
name = "bar"
version = "0.2.0"

[dependencies]
openzeppelin.workspace = true
```

:::

:::info
Paths used to declare path dependencies are relative to workspace root.
:::

### `[scripts]`

The `[scripts]` section can be used to define custom, cross-platform commands specific to the workspace codebase.
The values from the `[workspace.scripts]` table are available to be inherited by members with `{key}.workspace = true`.

Scripts are run for workspace member packages specified with `--package/--workspace` filters.

For example:

::: code-group

```toml [Scarb.toml]
[workspace]
members = ["foo"]

[workspace.scripts]
test = "snforge"
```

```toml [foo/Scarb.toml]
[package]
name = "foo"
version = "0.2.0"

[scripts]
test.workspace = true
```

:::

See [Scripts](./scripts) page for more information.

### `[tool]`

The `workspace.tool` table can be used for tools that would like to store configuration in `Scarb.toml`.
Similarly to the `[tool]` section from the package manifest,
the `[workspace.tool]` is not parsed by Scarb and will not be warned about.
The values from the `[workspace.tool]` table are available to be inherited by members with `{key}.workspace = true`.

For example:

::: code-group

```toml [Scarb.toml]
[workspace]
members = ["foo"]

[workspace.tool.snforge]
exit_first = true
```

```toml [foo/Scarb.toml]
[package]
name = "foo"
version = "0.2.0"

[tool]
snforge.workspace = true
```

:::

See [Tool](./manifest#tool) page for more information.

## `[profile]`

In a workspace context, only profiles defined in the root manifest are applied.

See [Profiles](./profiles) page for more information.
