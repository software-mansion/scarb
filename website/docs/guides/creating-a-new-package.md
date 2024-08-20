<script setup>
import { data as rel } from "../../github.data";
import {data as constants} from "../../constants.data";
</script>

# Creating a New Package

> [!TIP]
> At any time, you can review projects hosted in
> [example directory](https://github.com/software-mansion/scarb/tree/main/examples) from Scarb repository.

To start a new package with Scarb, use `scarb new`:

```shell
scarb new hello_world
```

The argument passed here is a name of the directory that Scarb will create.
It will also use it for package name.
To use a different package name, pass `--name your_package_name`.
This also initializes a new Git repository by default. If you don't want it to do that, pass `--no-vcs`.

As the result of running `scarb new`, Scarb has created two files:

- `Scarb.toml`
- `src/lib.cairo`

Let's take a closer look at `Scarb.toml`:

```toml-vue
[package]
name = "hello_world"
version = "0.1.0"
edition = "{{ constants.edition }}"

[dependencies]
```

This is called a **manifest**, and it contains all information that Scarb needs to compile your package.
This file is written in the [TOML](https://toml.io/) format.

The `src` directory contains the source code of your package, and the `lib.cairo` is the _main_ file.

Here's what's in `src/lib.cairo`:

```cairo filename="src/lib.cairo"
fn fib(a: felt252, b: felt252, n: felt252) -> felt252 {
    match n {
        0 => a,
        _ => fib(b, a + b, n - 1),
    }
}
```

Scarb generated a "hello world" code for us, a simple [Fibonacci](https://en.wikipedia.org/wiki/Fibonacci_number)
function that is exported by our package.
Let's compile it:

```shell
$ scarb build
   Compiling hello_world v0.1.0 (/path/to/package/hello_world/Scarb.toml)
    Finished release target(s) in 2 seconds
```

This will create a Sierra code of your program in `target/dev/hello_world.sierra.json`.

## Creating a Starknet package

To compile Starknet contracts, you need to add `starknet-contract` target and a `starknet` dependency to your manifest:

```toml-vue
[package]
name = "hello_world"
version = "0.1.0"
edition = "{{ constants.edition }}"

[dependencies]
starknet = "{{ rel.stable.starknetPackageVersionReq }}"

[[target.starknet-contract]]
```

The target definition will let Scarb know, that it should produce Starknet contract artifacts.
The `starknet` dependency tells Scarb to use a Starknet plugin during compilation of your contract.

Then, you can replace the `src/lib.cairo` file with your Starknet contract source code.
To compile it, simply run the same `build` command as you would for a regular Cairo package:

```shell
$ scarb build
   Compiling hello_world v0.1.0 (/path/to/package/hello_world/Scarb.toml)
    Finished release target(s) in 2 seconds
```

This will create a Sierra contract class artifact of your program in `target/dev/hello_world.contract_class.json`
that can be deployed to Starknet network.

### Creating a Starknet Foundry project

If you intend to use Starknet Foundry to test your contracts, you can create a Starknet Foundry project by
running:

```shell
scarb new hello_world --test-runner=starknet-foundry
```

This will create a Starknet package, with `snforge` already set up as your test runner. You can then execute `snforge`
tests by simply running:

```shell
scarb test
```

You can also build your package, like a regular Starknet package.
