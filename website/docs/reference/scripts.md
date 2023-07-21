# Scripts

Your package can define custom, cross-platform commands specific to a codebase.

## Defining commands

To define a custom command, add `[scripts]` section in your `Scarb.toml` file.
This consists of a mapping of command names to command definitions.
Each definition is a string that will be executed by the scripts shell.
For example:

```toml
[scripts]
foo = "echo 'Hello'"
bar = "echo 'World!'"
```

This section should not contain any values with type different from string, including subtables, arrays, or numbers.
In case the section is empty, it will be ignored.

## Listing scripts

To list all available scripts, you can use `scarb run` command.
It will list all available scripts and their definitions.

For example:

```shell
$ scarb run
Scripts available via `scarb run`:
bar                 : echo 'World!'
foo                 : echo 'Hello'
```

## Running scripts

`scarb run` uses a cross-platform shell that's a subset of sh/bash to execute defined tasks.

To run a script, use `scarb run <script>` command.

For example:

```bash
scarb run foo
```

The script definition from `Scarb.toml` file will be parsed and executed by the scripts shell.

### Working directory

The script will be run in the context of a package root.
You can specify the package to run within by using the package filter (`--package` or `-p`) argument.

### Environment variables

Environment variables are defined like the following:

```bash
export VAR_NAME=value
```

#### Predefined environmental variables

A set of predefined environmental variables will be passed to the running script.
The variables passed from the Scarb runtime are identical to the ones passed to Scarb subcommands.
See [Custom Subcomands - Environmental Variables](../writing-extensions/subcommands#environment-variables) for the
listing.

#### Shell variables

Shell variables are similar to environment variables, but won't be exported to spawned commands.
They are defined with the following syntax:

```bash
VAR_NAME=value
```

For example:
If you define:

```toml
[scripts]
foo = "USER=SWMANSION && export HELLO=Hello && echo $HELLO $USER!"
bar = "USER=SWMANSION && export HELLO=Hello && env"
```

Running the following will produce:

```shell
$ scarb run foo
Hello SWMANSION!
```

Although listing all commands with `env` from `bar` script will only include the "HELLO" variable, but not "USER".

### Built-in commands

The scripts shell ships with several built-in commands that work the same out of the box on Windows, Mac, and Linux.
Since the shell is based on [deno_tash_shell](https://crates.io/crates/deno_task_shell), you can learn more about this
mechanism from [deno docs](https://deno.land/manual@v1.31.3/tools/task_runner#built-in-commands).

### Using Scarb as a command

Your scripts can use `scarb` as a command, which will reference the scarb binary used to execute the script,
regardless of your system configuration (namely, we will not search the `PATH` variable).

## Acknowledgements

This functionality is based on [deno_task_shell](https://crates.io/crates/deno_task_shell) crate and the implementation
has been heavily influenced by the approach suggested by the [deno](https://github.com/denoland/deno) runtime team.
