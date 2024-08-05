# Creating a New Package

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

```toml
[package]
name = "hello_world"
version = "0.1.0"
edition = "2023_10"

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

This will create a Sierra code of your program in `target/release/hello_world.sierra.json`.
