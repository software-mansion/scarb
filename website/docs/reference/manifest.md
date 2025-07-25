<script setup>
import {data as constants} from "../../constants.data";
import { data as rel } from "../../github.data";
</script>

# The Manifest Format

The `Scarb.toml` file, present in each package, is called its _manifest_.
It is written in the [TOML](https://toml.io/) format.
It contains metadata that is needed to compile the package.
It has to be placed in the root of your project.
Use the `scarb manifest-path` command to locate the manifest used in current directory.

Every manifest file consists of the following sections:

## `[package]`

The first section in a `Scarb.toml` is `[package]`.

```toml
[package]
name = "hello_world" # the name of the package
version = "0.1.0"    # the current version, obeying semver
authors = ["Alice <a@example.com>", "Bob <b@example.com>"]
```

The only required fields are [`name`](#name) and [`version`](#version).
If publishing to a registry, it's recommended to fill in additional fields:

- [`license` or `license_file`](#license-and-license-file)
- [`description`](#description)
- [`homepage`](#homepage)
- [`documentation`](#documentation)
- [`repository`](#repository)
- [`readme`](#readme)

It would also be a good idea to include some [`keywords`](#keywords) and [`categories`](#ca), though they are not
required.

If package is not intended to be published, it is recommended to set [`publish`](#publish) field to false.

### `name`

The package name is a valid Cairo identifier used to refer to the package.
It is used when listed as a dependency in another package, and as the default name of targets.

The name must use only ASCII lowercase alphanumeric characters or `_`, and cannot be empty.
It also must not be a valid Cairo keyword or a wildcard pattern (`_`).

### `version`

Scarb bakes in the concept of [Semantic Versioning](https://semver.org/), so make sure you follow some basic rules:

1. Before you reach 1.0.0, anything goes, but if you make breaking changes, increment the minor version.
2. After 1.0.0, only make breaking changes when you increment the major version.
   Do not break the build.
3. After 1.0.0, don’t add any new public API in patch-level versions.
   Always increment the minor version if you add any new public structs, traits, fields, types, functions, methods,
   impls or anything else.
4. Use version numbers with three numeric parts such as 1.0.0 rather than 1.0.

### `edition`

The edition key is an optional key that affects which Cairo edition your package is compiled with.
The editions allow newer Cairo compiler versions to introduce opt-in features that may break existing code.
Setting the edition key in `[package]` will affect all targets in the package, including test suites etc.

```toml-vue
[package]
edition = "{{ constants.edition }}"
```

Most manifests have the edition field filled in automatically by `scarb new` with the latest available edition.
If the edition field is not present in Scarb.toml, then the default edition is assumed.

### `publish`

The publish field is an optional key that determines whether the package can be published to a registry.
Setting this field to false will prevent the package from being published.
By default, this field is set to `true`.

```toml
[package]
publish = true
```

### `cairo-version`

The `cairo-version` field is an optional key that tells Scarb what range of versions of the Cairo language and compiler
your package can be compiled with.
If the currently running version of the Scarb compiler does not match this requirement, Scarb will exit with an error,
telling the user what version is required.
This field takes a [semver version requirement](./specifying-dependencies#version-requirements).

```toml
[package]
cairo-version = "^{{ rel.preview.version }}"
```

Setting the `cairo-version` key in `[package]` will affect all targets in the package.

The value in this field will not affect the version of the compiler run by Scarb.
Scarb always uses its built-in version of the Cairo compiler.
It will instead show an error message to the user if the version of the Cairo compiler is not compatible with the project.

Checking Cairo version requirements can be skipped with `--ignore-cairo-version` argument.
Scarb will attempt to compile the project disregarding this field, even if it's not compatible with the builtin compiler version.

### `include`

When packaging a package with `scarb package` command (see
[packaging your package](../registries/publishing.md#packaging-your-package)), all files excluded with rules from
`.gitignore` or `.scarbignore` files are not included in the resulting package tarball.
This field can be used mark files and subdirectories that should be included in the package tarball, even if those files
would be excluded by rules from ignore files.
The paths are relative to the package root and cannot point to files outside the package.

```toml
[package]
include = ["target/some/file.txt"]
```

### `authors`

This optional field lists the people or organizations that are considered the "authors" of the package.
The exact meaning is open to interpretation - it may list the original primary authors, current maintainers, or owners
of the package.
An optional email address may be included within angled brackets at the end of each author entry.

```toml
[package]
authors = ["Software Mansion <contact@swmansion.com>", "Starkware"]
```

### `description`

The description is a short blurb about the package.
Package registries or indexers may display it with your package, some registries may even require it.
This should be plain text (not Markdown).

```toml
[package]
description = "A short description of my package."
```

### `documentation`

This field specifies a URL to a website hosting the crate's documentation.

```toml
[package]
documentation = "https://john.github.io/cairo-package"
```

### `readme`

This field should be the path to a file in the package root (relative to this `Scarb.toml`) that contains general
information about the package.

```toml
[package]
readme = "README.md"
```

If no value is specified for this field, and a file named `README.md`, `README.txt` or `README` exists in the package
root, then the name of that file will be used.
You can suppress this behavior by setting this field to false.
If the field is set to true, a default value of `README.md` will be assumed, unless file named `README.txt` or `README`
exists in the package root, in which case it will be used instead.

### `homepage`

This field should be a URL to a site that is the home page for your package.

```toml
[package]
homepage = "https://example.com/"
```

### `repository`

This field should be a URL to the source repository for your package.

```toml
[package]
repository = "https://github.com/software-mansion/scarb"
```

### `license` and `license-file`

The `license` field contains the name of the software license that the package is released under.
The `license-file` field contains the path to a file containing the text of the license (relative to this `Scarb.toml`).

Package registries must interpret the `license` field as
an [SPDX 2 license expression](https://spdx.github.io/spdx-spec/v2.3/SPDX-license-expressions/).
The license name must be a known license from the [SPDX license list](https://spdx.org/licenses/).

SPDX license expressions support AND and OR operators to combine multiple licenses.

```toml
[package]
license = "MIT OR Apache-2.0"
```

Using `OR` indicates the user may choose either license.
Using `AND` indicates the user must comply with both licenses simultaneously.
The `WITH` operator indicates a license with a special exception.
Some examples:

- `MIT OR Apache-2.0`
- `LGPL-2.1-only AND MIT AND BSD-2-Clause`
- `GPL-2.0-or-later WITH Bison-exception-2.2`

If a package is using a nonstandard license, then the `license-file` field may be specified instead of the `license`
field.

```toml
[package]
license-file = "LICENSE.txt"
```

### `keywords`

This field is an array of strings that describe your package.
This can help when searching for the package on a registry, and it is allowed to choose any words that would help
someone find this package.

```toml
[package]
keywords = ["account", "wallet", "erc-20"]
```

It is recommended that keywords are ASCII text, starting with a letter and only containing letters, numbers and `-`.
Additionally, keywords should have at most 20 characters, and single package should not provide more than 5 keywords.

### `urls`

This field is a map of additional internet links related to this package.
Keys are human-readable link names, and values are URLs.

```toml
[package.urls]
"We can help you build your project" = "https://swmansion.com/services/"
"We're hiring" = "https://swmansion.com/careers/"
```

### `experimental-features`

This field is responsible for setting experimental flags to be used on the package for the compiler.

```toml
[package]
experimental-features = ["negative_impls"]
```

### `re-export-cairo-plugins`

This field can be used to declare a re-export of a package dependency.
If a package declares a re-export, all packages depending on this package will also depend on the re-export.
Only packages implementing cairo plugin (with `cairo-plugin` target) can be re-exported.
Only direct dependencies can be re-exported.

```toml
[package]
re-export-cairo-plugins = ["proc_macro_package"]
```

## `[dependencies]`

See [Specifying Dependencies](./specifying-dependencies) page.

## `[dev-dependencies]`

See [Specifying Dependencies](./specifying-dependencies) page.

## Target tables: `[lib]` and `[[target]]`

See [Targets](./targets) page.

## `[cairo]`

Adjust Cairo compiler configuration parameters when compiling this package.
These options are not taken into consideration when this package is used as a dependency for another package.

> [!WARNING]
> In context of a workspace, only the `[cairo]` section from the workspace root manifest is applied.

The `[cairo]` section can be overridden by profile definition. See [Profiles](./profiles) page for more information.

### `sierra-replace-ids`

Replace all names in generated Sierra code with dummy counterparts, representing the
expanded information about the named items.

For libfuncs and types that would be recursively opening their generic arguments.
For functions, that would be their original name in Cairo.
For example, while the Sierra name be `[6]`, with this flag turned on it might be:

- For libfuncs: `felt252_const<2>` or `unbox<Box<Box<felt252>>>`.
- For types: `felt252` or `Box<Box<felt252>>`.
- For user functions: `test::foo`.

By default, this flag is set to `false`.

```toml
[cairo]
sierra-replace-ids = false
```

### `allow-warnings`

If enabled, Scarb will not exit with error on compiler warnings.
By default, this flag is set to `true`.

```toml
[cairo]
allow-warnings = true
```

### `enable-gas`

If set to `false`, during the project compilation Scarb will not add any instructions related to gas usage calculation.
Additionally, `gas: "disabled"` cfg attribute will be set.

By default, this flag is set to `true`.

```toml
[cairo]
enable-gas = true
```

This flag cannot be set to `false` while compiling the `starknet-contract` target.

### `inlining-strategy`

This field is responsible for setting the inlining strategy to be used by compiler when building the package.
The possible values are `default`, `avoid` or a numerical value.
If `avoid` strategy is set, the compiler will only inline function annotated with `#[inline(always)]` attribute.
By default, Scarb will use `avoid` strategy in `dev` profile and `default` strategy in `release` profile.

Example usage:

```toml
[cairo]
inlining-strategy = "avoid"
```

If numerical value is set, the compiler will inline functions up to the given weight.
Note, that the weight exact definition is a compiler implementation detail and is subject to changes with every release.
Example usage:

```toml
[cairo]
inlining-strategy = 18
```

> [!WARNING]
> Using the `avoid` strategy may result in a slower execution of the compiled code.
> Please use with caution, only if your tooling requires that.
> You can use profile settings overwriting, for more granular control of which builds use the avoid strategy.

### `panic-backtrace`

If enabled, during the project compilation Scarb will add panic backtrace handling to the generated code.
This can be useful for debugging purposes.
By default, this flag is set to `false`, as it won't be available on Starknet.

```toml
[cairo]
panic-backtrace = false
```

### `unsafe-panic`

If enabled, code for handling runtime panics of the compiled project will not be generated.
This might be useful for client side proving.
By default, this flag is set to `false`.

```toml
[cairo]
unsafe-panic = false
```

### `incremental`

If enabled, after project compilation Scarb will emit additional cache artifacts. This artifacts will be attempted to be
reused in subsequent builds.
By default, this flag is set to `true`.
This can also be disabled globally via `SCARB_INCREMENTAL` environment variable.

```toml
[cairo]
incremental = true
```

### `unstable-add-statements-functions-debug-info`

> [!WARNING]
> This is highly experimental and unstable feature intended to be used by [cairo-profiler], [cairo-coverage]
> and [forge] backtraces.
> It will slow down the compilation and cause it to use more system memory.
> It will also make the compilation artifacts larger.
> It should not be used unless your tooling requires it.

If enabled, during the project compilation Scarb will a add mapping between Sierra statement indexes and vectors of
fully qualified paths of Cairo functions to debug info. A statement index maps to a vector consisting of a function
which caused the statement to be generated and all functions that were inlined or generated along the way.
By default, this flag is set to `false`.

```toml
[cairo]
unstable-add-statements-functions-debug-info = false
```

### `unstable-add-statements-code-locations-debug-info`

> [!WARNING]
> This is highly experimental and unstable feature intended to be used by [cairo-profiler], [cairo-coverage]
> and [forge] backtraces.
> It will slow down the compilation and cause it to use more system memory.
> It will also make the compilation artifacts larger.
> It should not be used unless your tooling requires it.

If enabled, during the project compilation Scarb will add a mapping between Sierra statement indexes and locations in
the code to debug info. A statement index maps to a vector consisting of code fragment which caused the statement to be
generated and all code fragments that were inlined or generated along the way.
By default, this flag is set to `false`.

```toml
[cairo]
unstable-add-statements-code-locations-debug-info = false
```

## `[profile]`

> [!WARNING]
> In context of a workspace, only the profiles from the workspace root manifest are applied.

See [Profiles](./profiles) page.

## `[scripts]`

See [Scripts](./scripts) page.

## `[tool]`

> [!WARNING]
> In context of a workspace, the `[tool]` section still needs to be defined on the package to take effect.
> Packages can inherit `tool` section from workspace manifest, but only explicitly.
> See [Workspaces](./workspaces#tool) page for more detailed information.

> [!WARNING]
> Profiles can be used to change values defined in `[tool]` section.
> See [Profiles](./profiles#overriding-tool-metadata) page for more detailed information.

This section can be used for tools which would like to store package configuration in Scarb.toml.
Scarb by default will warn about unused keys in Scarb.toml to assist in detecting typos and such.
The `[tool]` table, however, is completely ignored by Scarb and will not be warned about.
For example:

```toml
[tool.snforge]
exit-first = true
```

No fields in this section are required or defined by Scarb.
Each field can accept any valid toml value including a table.

### `[tool.scarb]` section

Scarb defines own tool section, called `[tool.scarb]`, which can be used to store Scarb specific configuration.

As of now, only one key is supported in this section: `allow-prebuilt-plugins`.
This field accepts a list of names of packages from the dependencies of the package.
It can accept both direct and transient dependencies.
Adding a package name to this list means, that Scarb can load prebuilt plugins for this package and all of its dependencies.

Example usage:

```toml
[tool.scarb]
allow-prebuilt-plugins = ["snforge_std"]
```

> [!WARNING]
> Since loading a prebuilt plugin means loading and executing a binary file distributed with the dependency,
> only mark packages as allowed if you trust the source of the package and the package itself.
> Executing bytecode from untrusted sources can lead to security vulnerabilities.

This field is only read from the package that is currently build by Scarb.
Sections defined in dependencies will be ignored.

The prebuilt binaries are used in a best-effort manner - if it's not possible to load a prebuilt binary for any reason,
it will attempt to compile the macro source code instead.
No errors will be emitted if the prebuilt binary is not found or cannot be loaded.

See [prebuilt procedural macros](./procedural-macro.md#prebuilt-procedural-macros) for more information.

## `[workspace]`

See [Workspaces](./workspaces) page.

## `[features]`

See [Features](./conditional-compilation#features) page.

[cairo-profiler]: https://github.com/software-mansion/cairo-profiler
[cairo-coverage]: https://github.com/software-mansion/cairo-coverage
[forge]: https://github.com/foundry-rs/starknet-foundry
