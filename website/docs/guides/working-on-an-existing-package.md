# Working on an Existing Scarb Package

If you download an existing package that uses Scarb, it's really easy to get going.

First, get the package from somewhere.
For example, the [`alexandria`](https://github.com/keep-starknet-strange/alexandria) package is hosted on GitHub, and
we 'll clone its repository using Git.
Note that Alexandria is a collection of multiple packages, and we will use math as an example in this guide.

```shell
git clone https://github.com/keep-starknet-strange/alexandria
cd alexandria
cat Scarb.toml
```

Then to build it, use `scarb build`:

```shell
$ scarb build
   Compiling alexandria v0.1.0 (/path/to/package/alexandria/math/Scarb.toml)
    Finished release target(s) in 4 seconds
```

This will fetch all the dependencies and then build them, along with the package.
