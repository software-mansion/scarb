# Using cairo-run in Scarb projects

The `scarb cairo-run` command executes the `main` function of a local package.
It does automatically compile the cairo code within the package so using `scarb build` beforehand is not necessary.
This automatically called build can be skipped with the `--no-build` flag.
The extension accepts two optional parameters: `--available-gas` and `--print-full-memory`.
The first one is used to set the available gas for the execution.
If not provided, a gas usage is not limited.
Gas usage can be disallowed by setting the value to `0`.
The second one prints the full memory after the execution.

## Program arguments

The `main` function may take arguments. They can be passed to the `scarb cairo-run` command as a single JSON array of
numbers, decimal bigints or recursive arrays of those. Nested arrays can translate to either Cairo arrays, tuples or
plain structures. Consult the following table for example valid arguments and their matching function signatures:

| Argument              | Function signature                      |
| --------------------- | --------------------------------------- |
| `[1]`                 | `fn main(a: u64)`                       |
| `[1, "2"]`            | `fn main(a: u64, b: u64)`               |
| `[[1, 2], [3, 4, 5]]` | `fn main(t: (u64, u64), v: Array<u64>)` |
