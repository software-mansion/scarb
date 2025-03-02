<script setup>
import { data as rel } from "../../github.data";
</script>

# Scarb execute

The `scarb execute` command executes a function from a local package.
Only packages defining the [executable target](../reference/targets#executable-target) can be executed.
It does automatically compile the cairo code within the package so using `scarb build` beforehand is not necessary.
This automatically called build can be skipped with the `--no-build` flag.

## Choosing a function to run

If your package defines multiple main functions (see [choosing the main function](../reference/targets#choosing-the-main-function)),
you need to specify which executable target should be run.

This can be achieved through one of two flags:

- `--executable-name` to choose the target by its name.
- `--executable-function` to choose the target by the main function it defines.
  Those flags are mutually exclusive.

## Saving the execution information

The execution will be carried out for one of two execution targets: `standalone` or `bootloader`.
You can choose the target with the `--target` flag.
Standalone means that the program will be executed as-is, and intended to be proven directly with `scarb prove`.
hen we run with the bootloader target, the program’s execution is expected to be wrapped by the
[bootloader’s](https://github.com/Moonsong-Labs/cairo-bootloader?tab=readme-ov-file#cairo-bootloader) execution,
which itself will be proven via Stwo.

After the execution, information about it will be saved to the target directory of the package.

For `standalone` target, the output will be saved as trace files (`air_public_input.json`, `air_private_input.json`,
`memory.bin`, and `trace.bin`), which can be used for creating a proof with `scarb prove`.
For `bootloader` target, the output will be saved as a CairoPie format (Position Indenpendent Execution),
which is not yet supported by the `prove` command.

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

## Program arguments

The executable function may accept arguments.
They can be passed to the `scarb execute` command via either `--arguments` or `--arguments-file` flag.

The expected input with `--arguments` is a comma-separated list of integers.
This list should correspond to the Cairo’s Serde of main’s arguments, for example:

| main’s signature                       | valid arguments example |
| :------------------------------------- | :---------------------- |
| `fn main(num: u8)`                     | 1                       |
| `fn main(num1: u8, num2: u16)`         | 1,2                     |
| `fn main(num1: u8, tuple: (u16, u16))` | 1,2,3                   |
| `fn main(num1: u8, num2: u256)`        | 1,2,3                   |
| `fn main(num1: u8, arr: Array<u8>)`    | 1,2,1,2                 |

See the [documentation](https://docs.starknet.io/architecture-and-concepts/smart-contracts/serialization-of-cairo-types/) for more information about Cairo’s Serde.

Note that when using `--arguments-file`, the expected input is an array of felts represented as hex string.
For example, `1,2,3` in the above table becomes `["0x1", "0x2", "0x3"]`.
