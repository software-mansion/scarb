<script setup>
import { data as rel } from "../../github.data";
</script>

# Targets

Scarb packages consist of _targets_ which correspond to source files which can be compiled into a package, and the way
how this package is compiled.
Packages can have a built-in library target, and/or more externally defined targets.
Note, that dependencies adhere to packages and are independent of targets of the package.
When building a package, each target of this package will use exactly the same set of dependencies during compilation.
The list of targets can be configured in `Scarb.toml`.

## Overriding default targets

If the manifest does not list any targets, Scarb will assume the **library** target with its default parameters.

Specifying **any** target in the manifest file means that the default target will no longer be added.

In particular, this means that in order to use a package with contracts as a dependency, user must define
both `starknet-contract` and `lib` targets, like in the example below.

```toml
[lib]

[[target.starknet-contract]]

```

## Library

The library target defines a "library" that can be used by other packages.
In other words, if a package does not provide a library target, it cannot be used as a dependency.
If not specified, the name of the library defaults to the name of the package.
A package can have only one library target.

### Sierra and CASM code generation

The library target accepts following configuration parameters, with default values for the default _release_ profile:

```toml
[lib]
sierra = true        # Enable Sierra codegen.
casm = false         # Enable CASM codegen.
sierra-text = false  # Enable textual Sierra codegen.
```

By default, the library target builds unprocessed Sierra code in JSON form for the package.
When either the `casm` or `sierra-text` option is enabled, Scarb can automatically compile the Sierra code down to CASM
or textual Sierra, respectively.
While textual Sierra may be practical for debugging or similar tasks, relying on it in a production environment could
lead to unexpected behavior.

## Executable target

The executable target defines an artifact that can be executed with `scarb execute` command.
This target should be used for packages that are meant to be executed rather than used as dependencies.
See [Scarb execute](../extensions/execute.md) page for more details.
Execution of the package created by the `scarb execute` command can be proved with `scarb prove` and the proof can be
verified with `scarb verify`.
See [proving and verifying execution](../extensions/prove-and-verify.md) page for more details.

> [!TIP]
> The executable target requires the Cairo gas calculation to be disabled.
> Gas tracking introduces a computation overhead and makes less sense outside the context of Starknet smart contracts.
> You can achieve this with adding following section to your `Scarb.toml` manifest.
>
> ```toml
> [cairo]
> enable-gas = false
> ```
>
> See [manifest reference](../reference/manifest.md#cairo) for more details.

### Configurable properties

The executable target can define two custom properties: `allow-syscalls` and `function`.

The `function` property is described in detail in the [compiling with multiple main functions section](#compiling-with-multiple-main-functions).

The `allow-syscalls` property is a boolean that allows the package to use syscalls.
By default, this flag is set to `false`.

> [!WARNING]
> Execution of packages using syscalls cannot reliably be proven with `scarb prove` and `scarb verify`.
> You should only compile executable targets with syscalls if you do not need to prove its execution.

```toml
[cairo]
allow-syscalls = false
```

### Choosing the main function

To compile a package as an executable target, you need to declare which function should be the executable entry point.
This can be done by adding the `#[executable]` attribute to the function you want to use as the main function.
To make the `#[executable]` entrypoint available during the compilation, please add the `cairo_execute` package to the
dependencies.

```toml-vue
[dependencies]
cairo_execute = "{{ rel.stable.starknetPackageVersionReq }}"
```

Then add the attribute to the function of your choosing.

```cairo
#[executable]
fn main() {
    // Your code goes here
}
```

### Compiling with multiple main functions

If you need to execute the package multiple times, with different entry points, you can declare multiple executable
targets, each with a different main function.
This can be achieved with the `function` property of the executable target definition.

```toml
[executable]
function = "hello_world::main"

[[target.executable]]
name = "secondary"
function = "hello_world::secondary"
```

```cairo
#[executable]
fn main() -> felt252 {
    12
}

#[executable]
fn secondary() -> felt252 {
    34
}
```

Each of the executable targets defined will be compiled separately and produce separate artifacts.
Each of the targets need to have a unique name.
If your package defines `#[executable]` attribute multiple times, the `function` property is required to be set.

## Test targets

The test target produces artifacts that can be used by the `scarb cairo-test` or [Starknet Foundry] to run tests.
If your project implements Starknet contracts, the test target will compile them as well.
Each package can define multiple test targets, each of which will produce a separate test runner artifact.
The test runner relies on test target definitions to find runnable tests.

### Configurable properties

The test target can define three custom properties: `source-path`, `test-type` and `build-external-contracts`.
The `source-path` property is a path from package root, to the main Cairo file of the test module.
The `test-type` property accepts either `unit` or `integration` as a value, as described in
[tests organization](../extensions/testing#tests-organization).
The `build-external-contracts` allows compilation of contracts defined in dependencies of the tested package, as
described in [compiling external contracts](../extensions/starknet/contract-target#compiling-external-contracts) section
of the Starknet Contract Target page.

Example test target definition:

```toml
[[test]]
test-type = "unit"
```

Unlike other targets, test targets are not built by default.
To build test targets (and only test targets), use the `scarb build --test` command.

### Auto-detection of test targets

If your manifest file does not define any `[[test]]` sections, test targets will be automatically detected
from source files.
The following rules are used to detect test targets:

- A test target of `unit` type is added, with source path pointing to the main file of the package.
  The test target is named `{package_name}_unittest`.
- If there is a directory called `tests` in the package, besides a manifest file, it is searched for `integration`
  type test targets.
  - If the directory defines a `lib.cairo` file, a single test target with `source-path` pointing to it is created.
    The target will be named `{package_name}_tests`.
  - If the directory does not define a `lib.cairo` file, but contains other `.cairo` files, a test target is created
    for each of these files. The test targets will be named `{package_name}_{file_name}`.

## External targets

Scarb supports registering targets that are handled by Scarb extensions.
Such targets are called _external_ and are defined in a `[[target.*]]` array of tables.

:::info
This is not fully implemented, and we track this work in [#111](https://github.com/software-mansion/scarb/issues/111).
As for now, Scarb only supports internally hardcoded targets:

- [`starknet-contract`](../extensions/starknet/contract-target)

:::

If multiple targets of the same kind are defined in the package, they all must specify unique [names](#name).

## Configuring a target

All of the `[lib]`, `[test]` and `[[target.*]]` sections in `Scarb.toml` support configuration options that are not
target-specific and control how Scarb manages these targets.
The following is an overview of the TOML settings for each target, with each field described in detail below.

```toml
[lib]
name = "foo"  # The name of the target.
```

:::warning
Scarb reserves itself a right to introduce new global configuration fields in future versions. Potentially, new
parameters may end up being conflicting with ones accepted by external targets. Introducing new parameters will always
be done in major Scarb version bump, and will be loudly communicated earlier.
:::

### `name`

The `name` field specifies the name of the target, which corresponds to the filename of the artifact that will be
generated.
If missing, this defaults to the name of the package.
If multiple targets of the same kind are defined in the package, they all must specify unique names.

[Starknet Foundry]: https://foundry-rs.github.io/starknet-foundry/
