# Specifying Dependencies

Your packages can depend on other libraries from Git repositories or subdirectories on your local file system.

## Specifying dependencies from Git repositories

To depend on a package located in a Git repository, the minimum information needed to specify is the location of the
repository with the `git` key:

```toml
[dependencies]
alexandria_math = { git = "https://github.com/keep-starknet-strange/alexandria.git" }
```

Scarb will fetch the `git` repository at this location and then look for a `Scarb.toml` for the requested package
anywhere inside the Git repository
(not necessarily at the root of it - for example, if repository contains multiple packages in subdirectories).

Since no other information has been specified, Scarb assumes that it is intended to use the latest commit on the main
branch.
You can combine the `git` key with `branch`, `tag` and `rev` keys to specify something else.
Here is an example of specifying that you want to use the latest commit on a branch named `next`:

```toml
[dependencies]
alexandria_math = { git = "https://github.com/keep-starknet-strange/alexandria.git", branch = "next" }
```

Anything that is not a branch or tag falls under `rev`.
This can be a commit (short) hash, like `rev = "1f06df93"`, or a named reference exposed by the remote repository
such as `rev = "refs/pull/330/head"`.
What references are available varies by where the repository is hosted; GitHub in particular exposes a reference to the
most recent commit of every pull request as shown, but other Git hosts often provide something equivalent, possibly
under a different naming scheme.

## Specifying path dependencies

Scarb supports path dependencies, which are typically sub-packages that live within one repository.
To depend on a package located in a local directory, you need to specify the path to it, relative to
current `Scarb.toml`, with the `path` key:

```toml
[dependencies]
hello_utils = { path = "hello_utils" }
```

Scarb does not cache path dependencies, any changes made in them will be reflected immediately in builds of your
package.

## Development dependencies

In order to add development dependency, specify it under `[dev-dependencies]` section:

```toml
[dev-dependencies]
alexandria_math = { git = "https://github.com/keep-starknet-strange/alexandria.git" }
```

Development dependencies are not used when compiling a package for building, but are used for compiling tests.

These dependencies are not propagated to other packages which depend on this package.

## Version requirements

Scarb allows you to specify version requirements of dependencies with the `version` key:

```toml
[dependencies]
hello_utils = { version = "1.0.0", path = "hello_utils" }
```

The string `"1.0.0"` is a version requirement.
Although it looks like a specific version of the `hello_utils` package, it actually specifies a _range_ of versions and
allows [SemVer](https://semver.org/) compatible updates.
Scarb uses Rust's SemVer flavour, in a way implemented by the [`semver`](https://crates.io/crates/semver) crate.
An update is allowed if the new version number does not modify the left-most non-zero digit in the major, minor, patch
grouping.

Here are some more examples of version requirements and the versions that would be allowed with them:

```
1.2.3  :=  >=1.2.3, <2.0.0
1.2    :=  >=1.2.0, <2.0.0
1      :=  >=1.0.0, <2.0.0
0.2.3  :=  >=0.2.3, <0.3.0
0.2    :=  >=0.2.0, <0.3.0
0.0.3  :=  >=0.0.3, <0.0.4
0.0    :=  >=0.0.0, <0.1.0
0      :=  >=0.0.0, <1.0.0
```

This compatibility convention is different from SemVer in the way it treats versions before 1.0.0.
While SemVer says there is no compatibility before 1.0.0, Scarb considers `0.x.y` to be compatible with `0.x.z`,
where `y ≥ z` and `x > 0`.

It is possible to further tweak the logic for selecting compatible versions using special operators, though it shouldn't
be necessary most of the time.

### Caret requirements

Caret requirements are an alternative syntax for the default strategy, `^1.2.3` is exactly equivalent to `1.2.3`.

### Tilde requirements

Tilde requirements specify a minimal version with some ability to update.
If you specify a major, minor, and patch version or only a major and minor version, only patch-level changes are
allowed.
If you only specify a major version, then minor- and patch-level changes are allowed.

`~1.2.3` is an example of a tilde requirement.

```
~1.2.3  := >=1.2.3, <1.3.0
~1.2    := >=1.2.0, <1.3.0
~1      := >=1.0.0, <2.0.0
```

### Wildcard requirements

Wildcard requirements allow for any version where the wildcard is positioned.

`*`, `1.*` and `1.2.*` are examples of wildcard requirements.

```
*     := >=0.0.0
1.*   := >=1.0.0, <2.0.0
1.2.* := >=1.2.0, <1.3.0
```

### Comparison requirements

Comparison requirements allow manually specifying a version range or an exact version to depend on.

Here are some examples of comparison requirements:

```
>= 1.2.0
> 1
< 2
= 1.2.3
```

### Multiple requirements

As shown in the examples above, multiple version requirements can be separated with a comma, e.g., `>= 1.2, < 1.5`.
