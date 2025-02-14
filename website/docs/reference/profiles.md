# Profiles

Profiles provide a way to alter the compiler settings.

Scarb has 2 built-in profiles: `dev` and `release`.
The profile defaults to `dev` if not specified otherwise.
In addition to the built-in profiles, user can define custom profiles in the package manifest.
Profile can be specified through the command line arguments, or with an environment variable.

Profile can be changed in `Scarb.toml` manifest within the `[profile]` section.
Profiles defined in dependencies will be ignored.

Profile can alter the compiler settings (analog to manifest [`[cairo]`](./manifest#cairo) section) and custom tool
metadata (from [`[tool]`](./manifest#tool) section).

## Workspace profiles

In a workspace context, only profiles defined in the workspace root manifest are applied.
All profiles defined in the workspace members are ignored.
See [Workspaces](./workspaces) page for more information.

## Overriding built-in profile properties

Each of the built-in profiles come with predefined default properties.

The properties of a built-in profile can be overridden by specifying a new property value in a custom profile.

### Overriding Cairo compiler configuration

The Cairo compiler configuration is composed of `[cairo]` section and profiles defined in the manifest file.
The `[cairo]` section in the manifest can be used to override the default compiler settings.
Profiles can be used to further alter the compiler settings, by overriding values set in the `[cairo]` section.

For example, the `dev` profile has the `sierra-replace-ids` (see [`[cairo]`](./manifest#cairo)) property set to `true`
by default.
This can be overridden by specifying the same property in a custom profile:

```toml
[profile.dev.cairo]
# Replace all names in generated Sierra code with dummy counterparts.
sierra-replace-ids = true
```

### Overriding tool metadata

Tool metadata defined in the manifest (see [`[tool]`](./manifest#tool) can be overridden by a profile.

For example:

```toml
[tool.some-tool]
local = false
debug = false

[profile.dev.tool.some-tool]
debug = true
```

Note, that by default the subsection defined in the profile replaces one from `tool` section completely.
The config from above would be translated to:

```toml
[tool.some-tool]
debug = true
```

You can change the merge strategy with `merge-strategy` property.
For instance:

```toml
[tool.some-tool]
local = false
debug = false

[profile.dev.tool.some-tool]
merge-strategy = "merge"
debug = true
```

This would be translated to:

```toml
[tool.some-tool]
merge-strategy = "merge"
local = false
debug = true
```

Your tool config may be more complex than simple key-value pairs, for instance it can contain sub-tables.

```toml
[tool.some-tool]
top-level-key = "top-level-value"

[tool.some-tool.environment]
local = false
debug = false

[profile.dev.tool.some-tool.environment]
debug = true
```

In such case, same principles apply for merging sub-tables.
The config from above would be translated to:

```toml
[tool.some-tool.environment]
debug = true
```

If you want to merge sub-tables, you can specify `merge-strategy` property like following snippet.

```toml
[tool.some-tool]
top-level-key = "top-level-value"

[tool.some-tool.environment]
local = false
debug = false

[profile.dev.tool.some-tool]
merge-strategy = "merge"

[profile.dev.tool.some-tool.environment]
merge-strategy = "merge"
debug = true
```

This would be translated to:

```toml
[tool.some-tool]
merge-strategy = "merge"
top-level-key = "top-level-value"

[tool.some-tool.environment]
merge-strategy = "merge"
local = false
debug = true
```

## Defining custom profiles

Custom profiles can be defined in Scarb.toml with the `[profile]` table.
Each profile is defined by a name and a set of properties.

For example, the following defines a custom profile named `my-profile`:

```toml
[profile.my-profile]
```

A custom profile can be used with `--profile` argument. For instance:

```shell
scarb --profile my-profile build
```

### Profile inheritance

Each custom profile inherits the properties of one of the built-in profiles.
The built-in profile to inherit from is specified with the `inherits` property.

For example:

```toml
[profile.my-profile]
inherits = "release"
```

If not specified, the `dev` profile is used by default.
A custom profile can override properties of the inherited profile, analogous to how built-in profile properties can be
overridden.
