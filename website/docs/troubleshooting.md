# Troubleshooting

It is possible that you encounter some issues while working with Scarb.
This page lists some common issues and their possible solutions.
Before reporting an issue to the Scarb team, please make sure to check the following list.

## Stack overflow

In case of a bug in the Cairo compiler implementation, it may consume too much stack space in some specific cases.
Usually, this happens while compiling large Cairo codebases.
This often results in an error message like:

```
thread 'main' has overflowed its stack
fatal runtime error: stack overflow
Aborted (core dumped)
```

Usually it does not seem to consume infinite amounts though, so you can try to confine it in an arbitrarily chosen
big memory chunk.

To run the Cairo compiler with a bigger stack size, you can use the `RUST_MIN_STACK` environmental variable.
For example, to set the stack size to 128MB, you can run:

```bash
RUST_MIN_STACK=134217728 scarb build
```

Please note that this is a workaround and not a permanent solution.
If you encounter this issue, please report it to the compiler team at [Cairo issues].

## Procedural macros undefined symbol

When compiling a project that uses procedural macros, if you encounter an error message like this:

```
undefined symbol: __start_linkm2_MACRO_DEFINITIONS_SLICE
```

You can try following workarounds:

1. Make sure that you have a stable Cargo release installed. You can check the installed version by
   running `cargo --version`.
2. If the error still persists on stable Cargo release, please try running build with
   either `RUSTFLAGS="-C link-dead-code"` or `RUSTFLAGS="-C link-args=-znostart-stop-gc"` flags. You can submit the
   flags to the compiler by pasting them before the `scarb build` command in your terminal.

[Cairo issues]: https://github.com/starkware-libs/cairo/issues/
