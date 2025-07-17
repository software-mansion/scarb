<script setup>
import { data as rel } from "../../github.data";
import {data as constants} from "../../constants.data";
</script>

## Defining an Executable Package

Start a new Scarb project with `scarb new <project_name>`.
In your `Scarb.toml` file:

1. Set the package to compile to a Cairo executable by adding `[executable]` (note that `lib` or `starknet-contract` targets cannot be executed in this way).
2. Add the `cairo_execute="{{ rel.stable.starknetPackageVersionReq }}"` plugin to your dependencies.
3. Disable gas usage by adding `enable-gas = false` under the `[cairo]` section (gas is only supported for `lib` or `starknet-contract` targets).

Below we have an example of the manifest file of a simple executable

```toml-vue
[package]
name = "test_execute"
version = "0.1.0"
edition = "{{ constants.edition }}"

[[target.executable]]

[cairo]
enable-gas = false

[dependencies]
cairo_execute = "{{ rel.stable.starknetPackageVersionReq }}"
```

Now we can move on to the code itself. An executable project must have **exactly one function** annotated with the `#[executable]` attribute. Consider the following simple `lib.cairo` file of an executable project:

```cairo
#[executable]
fn main(num: u8) -> u8 {
    num
}
```

You can now run:

```shell
scarb execute -p test_execute --print-program-output --arguments 5
```

Where `test_execute` is the name of the package with the executable target (as defined in our Scarb.toml manifest).

The above command runs our executable function within the `test-execute` package and prints the program's output segment.

The execution information will be saved under the `target/execute/<target name>` directory.
For each execution, a new output directory will be created, with consecutive number as names (e.g. `execution1`, `execution2`, ...).
To clean the target directory, you can use the `scarb clean` command.

For more information see detailed [`scarb execute`](../extensions/execute.md) documentation.
