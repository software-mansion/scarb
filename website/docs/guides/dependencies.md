# Managing dependencies

Scarb manages dependencies by cloning packages from their Git repositories.
To add a dependency, simply declare it in your `Scarb.toml`.

> [!WARNING]
> Using Git repositories as a foundation for package management is not the recommended approach anymore.
> Instead, [registries](../registries/overview.md) are now the primary way to manage dependencies.
> For details on how to specify dependencies from the official registry,
> see [here](../reference/specifying-dependencies#specifying-dependencies-from-official-registry).
> If you want to rely on git dependencies instead of the recommended way, you can learn more
> [here](../reference/specifying-dependencies#specifying-dependencies-from-git-repositories).

## Adding a dependency

If your `Scarb.toml` doesn't already have a `[dependencies]` section, add it, then list the package name and the version
required. You can search for packages to use through the [scarbs.xyz registry website](https://scarbs.xyz/).
This example adds a dependency on the [`openzeppelin_merkle_tree`](https://github.com/OpenZeppelin/cairo-contracts)
package (note that OpenZeppelin is a collection of multiple packages, and we will use only one of them as an example in
this guide). To see all available versions of some package, you can see the versions pane on the package's
[registry page](https://scarbs.xyz/packages/openzeppelin_merkle_tree). At the time of writing this guide, the latest
version of the `openzeppelin_merkle_tree` package is `0.17.0`, which is the version we will use.

```toml
[dependencies]
openzeppelin_merkle_tree = "0.17.0"
```

Using `"0.17.0"` as version requirement means, that you want to use a version `0.17.0` or newer, up until `0.18.0`. To
accept only a specific version, you can use `"=0.17.0"`. You can learn more about specifying version
requirements [here](../reference/specifying-dependencies#version-requirements)

Note, that if you want to add more dependencies, you do not have to add `[dependencies]` for each package separately.
For example:

```toml
[dependencies]
openzeppelin_merkle_tree = "0.17.0"
openzeppelin_account = "0.17.0"
```

Now, run `scarb build`, and Scarb will fetch new dependencies and all of their dependencies.
Then it will compile your package with all of these packages included:

```shell
$ scarb build
 Downloading openzeppelin_account v0.17.0
 Downloading openzeppelin_merkle_tree v0.17.0
 Downloading openzeppelin_utils v0.17.0
 Downloading openzeppelin_introspection v0.17.0
   Compiling hello_world v0.1.0 (/path/to/package/hello_world/Scarb.toml)
    Finished `dev` profile target(s) in 4 seconds
```

Note that the dependencies of specified packages are also downloaded during the build process.

You can now use the `openzeppelin_merkle_tree` package in `src/lib.cairo`:

```cairo
use openzeppelin_merkle_tree::hashes::PedersenCHasher;
fn hash() {
    let a = 'a';
    let b = 'b';
    let _hash = PedersenCHasher::commutative_hash(a, b);
}
```

## Development dependencies

You can add a `[dev-dependencies]` section to your Scarb.toml whose format is equivalent to `[dependencies]`:

```toml
[dev-dependencies]
openzeppelin_merkle_tree = "0.17.0"
```

## Adding a dependency via `scarb add`

If you prefer, you can also ask Scarb to edit `Scarb.toml` to add a dependency automagically for you.
The `scarb add` command accepts many parameters, matching all possibilities of expressing dependencies.
It can also automatically keep the list sorted, if it already is.
For example, the above example of dependency on `openzeppelin_merkle_tree`, can be also added like this:

```shell
scarb add openzeppelin_merkle_tree@0.17.0
```

You can add development dependencies similarly by passing `--dev` flag:

```shell
scarb add --dev openzeppelin_merkle_tree@0.17.0
```

You can also use it to add git commands if you wish:

```shell
scarb add openzeppelin_merkle_tree --git https://github.com/OpenZeppelin/cairo-contracts.git --tag 0.17.0
```

## Removing a dependency

To remove a dependency, simply remove related lines from your `Scarb.toml`.

As a quick shortcut, the `scarb remove` (also available in short `scarb rm`) can clean the manifest automatically:

```shell
scarb rm openzeppelin_merkle_tree
```

Removing development dependencies, like in `scarb add`, requires passing `--dev` flag:

```shell
scarb rm --dev openzeppelin_merkle_tree
```
