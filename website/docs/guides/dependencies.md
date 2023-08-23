# Managing dependencies

Scarb manages dependencies by cloning packages from their Git repositories.
To add a dependency, simply declare it in your `Scarb.toml`.

::: info
Using Git repositories as a foundation for package management is not an ideal
approach. Therefore, we plan to create a proper package registry in long term.
:::

## Adding a dependency

If your `Scarb.toml` doesn't already have a `[dependencies]` section, add it, then list the package name and the URL to
its Git repository.
This example adds a dependency on the [`alexandria`](https://github.com/keep-starknet-strange/alexandria) package (note
that Alexandria is a collection of multiple packages, and we will use `alexandria_math` as an example in this guide):

```toml
[dependencies]
alexandria_math = { git = "https://github.com/keep-starknet-strange/alexandria.git" }
```

In fact, it is always good to pin Git dependencies to concrete commits, otherwise Scarb would try to update this
dependency each time it is executed.
You can achieve this using one of the following extra fields that you can pass along `git`: `branch`, `tag` and `rev`.

Actually this is how the OpenZeppelin Contracts for Cairo library is released, since the `main` branch is not stable.

```toml
[dependencies]
openzeppelin = { git = "https://github.com/OpenZeppelin/cairo-contracts.git", tag = "v0.7.0-rc.0" }
```

::: info
In the future commit pinning will not be needed because Scarb will maintain a lockfile.
We track this feature in this issue: [#126](https://github.com/software-mansion/scarb/issues/126).
:::

Note, that if you want to add more dependencies, you do not have to add `[dependencies]` for each package separately. For example:

```toml
[dependencies]
alexandria_math = { git = "https://github.com/keep-starknet-strange/alexandria.git" }
openzeppelin = { git = "https://github.com/OpenZeppelin/cairo-contracts.git", tag = "v0.7.0-rc.0" }
```

Now, run `scarb build`, and Scarb will fetch new dependencies and all of their dependencies.
Then it will compile your package with all of these packages included:

```shell
$ scarb build
    Updating git repository https://github.com/keep-starknet-strange/alexandria
    Updating git repository https://github.com/OpenZeppelin/cairo-contracts
   Compiling hello_world v0.1.0 (/path/to/package/hello_world/Scarb.toml)
    Finished release target(s) in 4 seconds
```

You can now use the `alexandria_math` package in `src/lib.cairo`:

```cairo
use alexandria_math::fibonacci;
fn main() -> felt252 {
    fibonacci::fib(0, 1, 10)
}
```

## Adding a dependency via `scarb add`

If you prefer, you can also ask Scarb to edit `Scarb.toml` to add a dependency automagically for you.
The `scarb add` command accepts many parameters, matching all possibilities of expressing dependencies.
It can also automatically keep the list sorted, if it already is.
For example, the above example of dependency on `alexandria_math`, can be also added like this:

```shell
scarb add alexandria_math --git https://github.com/keep-starknet-strange/alexandria.git --rev 27fbf5b
```

## Removing a dependency

To remove a dependency, simply remove related lines from your `Scarb.toml`.

As a quick shortcut, the `scarb remove` (also available in short `scarb rm`) can clean the manifest automatically:

```shell
scarb rm alexandria_math
```
