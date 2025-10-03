<script setup>
import { data as rel } from "../../github.data";
</script>

# Scarb execute

The `scarb execute` command executes a function from a local package.
It does automatically compile the Cairo code within the package so using `scarb build` beforehand is not necessary.
If `scarb build` or `scarb execute` has been previously used and the package hasn't changed since,
this automatic build can be optionally skipped with the `--no-build` flag.
Only packages defining the [executable target](../reference/targets#executable-target) can be executed.

## Choosing a function to run

If your package defines multiple main functions (see [choosing the main function](../reference/targets#choosing-the-main-function)),
you need to specify which executable target should be run.

This can be achieved through one of two flags:

- `--executable-name` to choose the target by its name.
- `--executable-function` to choose the target by the main function it defines.

Those flags are mutually exclusive.

## Saving the execution information

The execution will be carried out for one of two execution targets: `standalone` or `bootloader`.
You can choose the target with the `--target` flag:

- **Standalone**: executes program as-is, execution is intended to be directly proven with `scarb prove`.
- **Bootloader**: program’s execution is expected to be wrapped by
  the [bootloader’s](https://github.com/Moonsong-Labs/cairo-bootloader?tab=readme-ov-file#cairo-bootloader) execution,
  which itself will be proven via Stwo.

See more on [proving and verifying execution](./prove-and-verify.md) page.

## Resource usage and program output

To print the Cairo program output, you can use the `--print-program-output` flag.
Otherwise, the output will be discarded.

To print detailed execution resources usage, you can use the `--print-resource-usage` flag.
This will show information about:

- `n_steps`
- `n_memory_holes`
- `builtin_instance_counter`
- `syscalls`

In case your Cairo program panics, the panic reason will be shown on the output, and the program will exit with a
non-zero exit code.

## Program arguments

The executable function may accept arguments.
They can be passed to the `scarb execute` command via either `--arguments` or `--arguments-file` flag.

The expected input with `--arguments` is a comma-separated list of integers.
This list should correspond to the Cairo’s Serde of main’s arguments, for example:

| main’s signature                       | valid arguments example | valid arguments file contents example |
| :------------------------------------- | :---------------------- | :------------------------------------ |
| `fn main(num: u8)`                     | 1                       | ["0x1"]                               |
| `fn main(num1: u8, num2: u16)`         | 1,27                    | ["0x1", "0x1b"]                       |
| `fn main(num1: u8, tuple: (u16, u16))` | 1,2,27                  | ["0x1", "0x2", "0x1b"]                |
| `fn main(num1: u8, num2: u256)`        | 1,2,27                  | ["0x1", "0x2", "0x1b"]                |
| `fn main(num1: u8, arr: Array<u8>)`    | 1,2,1,2                 | ["0x1", "0x2", "0x1", "0x2"]          |

Note that when using `--arguments-file`, the expected input is an array of felts represented as hex string.
See the [documentation](https://docs.starknet.io/architecture-and-concepts/smart-contracts/serialization-of-cairo-types/) for more information about Cairo’s Serde.

## Profiling your cairo program

To effectively analyze the performance of your Cairo program, you can use the
[cairo-profiler](https://github.com/software-mansion/cairo-profiler) tool.

Before profiling, you need to generate a trace data file that the profiler can read.
In order to do that, you can use the `--save-profiler-trace-data` flag.

:::warning
cairo-profiler depends on generated sierra code to get function mappings. Make sure to set `sierra = true` in your
`[executable]` target in Scarb.toml.
:::

For detailed usage instructions on how to use the cairo-profiler, please consult its
[documentation](https://github.com/software-mansion/cairo-profiler?tab=readme-ov-file#generating-output-file).

### Tracked resource

By default, `scarb execute` allows to profile your Cairo code's cairo steps samples. It is, however, possible to profile
based on sierra gas samples. The following is a TOML setting that allows to achieve that.

```toml
[tool.cairo-profiler]
tracked-resource = "sierra-gas"
```

:::info
This setting influences the contents of cairo-profiler trace data file created by scarb, not the behaviour of the
cairo-profiler itself. Therefore, after setting this in your Scarb.toml, make sure to execute your Cairo program once
again to generate a new trace file that will allow tracking the sierra gas sample.
:::
