# Working on an Existing Scarb Package

If you download an existing package that uses Scarb, it's really easy to get going.

First, get the package from somewhere.
For example, the [`alexandria`](https://github.com/keep-starknet-strange/alexandria) package is hosted on GitHub, and
we 'll clone its repository using Git.
Note that Alexandria is a collection of multiple packages, and we will use `alexandria_math` as an example in this
guide.

```shell
git clone https://github.com/keep-starknet-strange/alexandria
cd alexandria
cat Scarb.toml
```

Then to build it, use `scarb build`:

```shell
$ scarb build
   Compiling alexandria_ascii v0.1.0 (/path/to/package/alexandria/src/ascii/Scarb.toml)
   Compiling alexandria_data_structures v0.1.0 (/path/to/package/alexandria/src/data_structures/Scarb.toml)
   Compiling alexandria_encoding v0.1.0 (/path/to/package/alexandria/src/encoding/Scarb.toml)
   Compiling alexandria_linalg v0.1.0 (/path/to/package/alexandria/src/linalg/Scarb.toml)
   Compiling alexandria_math v0.2.0 (/path/to/package/alexandria/src/math/Scarb.toml)
   Compiling alexandria_numeric v0.1.0 (/path/to/package/alexandria/src/numeric/Scarb.toml)
   Compiling alexandria_searching v0.1.0 (/path/to/package/alexandria/src/searching/Scarb.toml)
   Compiling alexandria_sorting v0.1.0 (/path/to/package/alexandria/src/sorting/Scarb.toml)
   Compiling alexandria_storage v0.2.0 (/path/to/package/alexandria/src/storage/Scarb.toml)
    Finished release target(s) in 5 seconds
```

This will fetch all the dependencies and then build them, along with the package.
