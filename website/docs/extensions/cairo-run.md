<script setup>
import { data as rel } from "../../github.data";
</script>

# Using cairo-run in Scarb projects

The `scarb cairo-run` command executes a function from a local package.
It does automatically compile the cairo code within the package so using `scarb build` beforehand is not necessary.
This automatically called build can be skipped with the `--no-build` flag.
The extension accepts two optional parameters: `--available-gas` and `--print-full-memory`.
The first one is used to set the available gas for the execution.
If not provided, a gas usage is not limited.
Gas usage can be disallowed by setting the value to `0`.
The second one prints the full memory after the execution.

## Choosing a function to run

In general, a function to run can be specified in two ways:

- by the name of the function, with `--function` cli argument
- with the `#[main]` attribute, provided by the `cairo_run` package.

You can enable the `#[main]` attribute in your project by
adding `starknet = "{{ rel.stable.starknetPackageVersionReq }}"` to the dependencies section of your Scarb manifest.
If you do not add the `cairo_run` package to your dependencies - it's required to build the project
with [`sierra-replace-ids`](../reference/manifest#sierra-replace-ids) flag enabled.
You can also provide a function name argument with `--function` flag.

The precedense of the function to run is as follows:

1. If a `#[main]` attribute is specified on a function, it will be run.
2. If more than one function is marked with the `#[main]` attribute and debug names (`sierra-replace-ids`) are enabled,
   the function name must be provided with the `--function` flag.
3. If more than one function is marked with the `#[main]` attribute and debug names are disabled, an error is produced.
4. If there is no `#[main]` attribute and debug names are enabled, the function specified with the `--function` flag
   will be run.
5. If not specified and debug names are enabled, a function called `main` will be run.
6. If neither `#[main]` attribute nor debug names are enabled, an error is produced.

This way you can execute a function even without building your package with debug names enabled.

## Program arguments

The `main` function may take arguments. They can be passed to the `scarb cairo-run` command as a single JSON array of
numbers, decimal bigints or recursive arrays of those. Nested arrays can translate to either Cairo arrays, tuples or
plain structures. Consult the following table for example valid arguments and their matching function signatures:

| Argument               | Function signature                                                                                                              |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `[1]`                  | `fn main(a: u64)`                                                                                                               |
| `[1, "2"]`             | `fn main(a: u64, b: u64)`                                                                                                       |
| `[1, 2, [3, 4, 5]]`    | `fn main(t: (u64, u64), v: Array<u64>)`                                                                                         |
| `[1, 2, 3, [3, 4, 5]]` | <pre>struct Input {<br/> a: felt252,<br/> b: felt252,<br/> c: felt252,<br/>}<br/>fn main(t: Input, v: Array\<u64>)</pre>        |
| `[1, 2, 3, 4, 5]`      | <pre>struct Input {<br/> a: felt252,<br/> b: felt252,<br/> c: felt252,<br/>}<br/>fn main(t: Input, v: (felt252, felt252))</pre> |
| `[[1, 2, 3]]`          | <pre>struct Input {<br/> a: Array\<felt252>,<br/>}<br/>fn main(t: Input)</pre>                                                  |
