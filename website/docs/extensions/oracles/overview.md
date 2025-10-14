# Oracles <Badge type="warning" text="experimental" />

> [!WARNING]
> This is an experimental feature. The API and behaviour may change in future versions of Scarb.
> Oracles are currently available in **`scarb execute`** and `scarb cairo-test` with the `--experimental-oracles` flag.
> Support is also planned in future versions of **`snforge`**.

An oracle is an external process (like a script, binary, or web service) that exposes custom logic or data to a Cairo
program at runtime. You use it to perform tasks the Cairo VM can't, such as accessing real-world data or executing
complex, non-provable computations.

## Using oracles

The `oracle` library provides a type-safe interface for interacting with external oracles in Cairo applications.
Invoking oracles via this package is the recommended way, as it provides a well-tested, secure, and maintainable
interface for oracle interactions.

<BigLink href="https://scarbs.xyz/packages/oracle">
  Go to oracle on scarbs.xyz
</BigLink>

The documentation for this package provides a bird's-eye overview, guidelines, and instructions on how to invoke oracles
from Cairo code.

<BigLink href="https://docs.swmansion.com/cairo-oracle">
  Go to oracle documentation
</BigLink>

In the [oracle repository](https://github.com/software-mansion/cairo-oracle) there is also an end-to-end example
showcasing the feature. It implements a simple Cairo executable script, that invokes an oracle written in Rust that
runs as a child process.

<BigLink href="https://github.com/software-mansion/cairo-oracle/tree/main/example">
  Go to oracle example
</BigLink>

## Writing oracles

In Cairo, the oracle abstraction doesn't specify how to implement or execute oracles. Those are details that are
specific to the executor being used. Oracles are invoked using generic connection strings with the following format:

```
protocol:connection params
```

The [Scarb executor](../../extensions/execute.md) supports multiple oracle protocols:

- [`shell`](./shell.md) — one‑shot shell command execution returning stdout.
- [`wasm`](./wasm.md) — run WebAssembly components.

Some oracles may depend on external files. Use the [`assets`](../../reference/manifest.md#assets) field to include them
in the build.
