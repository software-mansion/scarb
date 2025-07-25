# The `Scarb.lock` lockfile

_Lockfiles_ are a common mechanism that allows package manager to lock dependencies to concrete versions, ensuring
reproducible builds.

## Scarb.toml vs Scarb.lock

`Scarb.toml` and `Scarb.lock` serve two different purposes.

`Scarb.toml` is about describing your dependencies in a broad sense, and is written by you.
`Scarb.lock` is a file that captures the exact version of every dependency used in a workspace or package.
It is maintained by Scarb and should not be manually edited.

## Why reproducible builds are important?

When [specifying project dependencies](specifying-dependencies.md), rather than pointing to specific versions of
packages you want to rely on, you define version requirements.  
Usually, those requirements can accept multiple versions.
For instance, specifying `1.2.3` would allow all versions `>=1.2.3` and `<2.0.0`.
Similarly, the following specification can be resolved to more than one commit hash:

```toml
alexandria_math = { git = "https://github.com/keep-starknet-strange/alexandria.git", branch = "next" }
```

Without lockfile mechanism, Scarb would always pull the latest commit from the branch `next`, which may change between
Scarb runs.
Consequently, we would not be able to ensure that Scarb builds are reproducible, i.e. two subsequent `scarb build`
calls would produce the same results using the same source code.
This is not ideal, as some regressions or incompatibilities may be introduced in the new changes of packages from
dependency.
Additionally, for security reasons users should be able to control what changes are pulled to their builds.
Consequently, users often would have to manually lock their dependency revisions, e.g. by specifying the commit hash from
the dependency package repository in the manifest file, which is tedious and error-prone.

## How lockfiles work?

Lockfiles automatically lock dependencies to a certain revisions, by writing resolved versions to a file beside
the project manifest.
This file is called `Scarb.lock`.
It's then read before Scarb resolves dependencies, and locked versions are used by the resolver.
If you change your dependencies specification in the manifest file, lockfile will change as well.
Lockfiles contain exact specifications of all packages from full tree of dependencies (including dependencies of
dependencies etc.).
Scarb lockfiles can easily be reviewed by the user.

> [!IMPORTANT]
> Lockfiles should be committed to version control system (e.g. a Git repository),
> allowing for full tracking of concrete version changes.

## Lockfile format

The lockfile is a TOML file, which starts with comment containing an auto-generated file marker and a version field.
The version field is used to distinguish between different lockfile formats.
For now, this will always state `1`.

Then, a list of package metadata entries are printed as TOML objects.
The list is sorted alphabetically by package name.
Each entry contains the following fields:

- `name` - name of the package, as in [Scarb.toml manifest](./manifest.md#name)
- `version` - version of the package, as in [Scarb.toml manifest](./manifest.md#version)
- `source` - the string representation of the source of the package.
- `dependencies` - a list of names of packages that this package depend on.
  This field is omitted if the package has no dependencies.

Note that each package can be listed only once, even if it is used by multiple other packages.
This is a direct consequence of the fact, that Cairo compilation model does not accommodate multiple versions
of the same package.

For instance, if a package with following manifest is created:

```toml
[package]
name = "hello_world"
version = "0.1.0"

[dependencies]
alexandria_math = { git = "https://github.com/keep-starknet-strange/alexandria.git" }
alexandria_data_structures = { git = "https://github.com/keep-starknet-strange/alexandria.git" }
```

The resulting lockfile will look like this:

```toml
# Code generated by scarb DO NOT EDIT.
version = 1

[[package]]
name = "alexandria_data_structures"
version = "0.1.0"
source = "git+https://github.com/keep-starknet-strange/alexandria.git#3356bf0c5c1a089167d7d3c28d543e195325e596"

[[package]]
name = "alexandria_math"
version = "0.2.0"
source = "git+https://github.com/keep-starknet-strange/alexandria.git#3356bf0c5c1a089167d7d3c28d543e195325e596"
dependencies = [
 "alexandria_data_structures",
]

[[package]]
name = "hello_world"
version = "0.1.0"
dependencies = [
 "alexandria_data_structures",
 "alexandria_math",
]
```

## Updating locked versions

To update all versions locked by the lockfile, run `scarb update` command.
This will perform project resolution ignoring the existing lockfile, then write out a new `Scarb.lock`
with the new version information.
Note that the `Scarb.toml` manifest file will not be changed.
If the project previously used any "yanked" versions (i.e., versions that have been marked as unusable for reasons such as critical bugs or security vulnerabilities), the `scarb update` command will invalidate its usages.
