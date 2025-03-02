<script setup>
import { data as rel } from "../../github.data";
import {data as constants} from "../../constants.data";
</script>

## Defining an Executable Package

Start a new Scarb project with `Scarb new <project_name>`, and add the following to your `Scarb.toml` file:

1. Specify that this package should compile to a Cairo executable by adding `[[target.executable]]` to your toml file (note that `lib` or `starknet-contract` targets cannot be executed in this way)
2. Add the `cairo_execute="{{ rel.stable.starknetPackageVersionReq }}"` plugin to your dependencies
3. Disable gas usage (gas is only supported for `lib` or `starknet-contract` targets) by adding `enable-gas = false` under the `[cairo]` section in your toml.

Below we have an example of the manifest file of a simple executable

```
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

```
#[executable]
fn main(num: u8) -> u8 {
    num
}
```

You can now run:

```
scarb execute -p test_execute --print-program-output --arguments 5
```

Where `test_execute` is the name of the package with the executable target (as defined in our Scarb.toml manifest)

The above command runs our executable function within the `test-execute` package and prints the program's output segment, which contains a success bit (0 for success) followed by the Cairo Serde of main’s output or the panic reason in case of a panic.
