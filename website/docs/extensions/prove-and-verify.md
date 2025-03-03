# Proving and verifying execution

> [!WARNING]
> Soundness of the proof is not yet guaranteed by Stwo, use at your own risk!

> [!WARNING]
> The prover is not available on Windows. Sorry for your inconvenience.

> [!WARNING]
> The `stwo-cairo` prover can be significantly slower when used through Scarb.
> See [performance](#Performance) section.

Scarb integrates the [`stwo-cairo` prover](https://github.com/starkware-libs/stwo-cairo) which can be used through `scarb prove`
and `scarb verify` commands.

## Proving Cairo execution

Only packages defining the [executable target](../reference/targets.md/#Executable-target) can be proven.
To prove the execution, you need to run the `scarb execute` command first, which will save execution information under the `target/execute/<target name>` directory.
For each execution, a new output directory will be created, with consecutive number as names (e.g. `execution1`, `execution2`, ...).
To clean the target directory, you can use the `scarb clean` command.

To prove the execution, you can run:

```shell
scarb prove --execution-id <index of the relevant execution>
```

You can also run `scarb prove` with the `--execute` flag, which will run the `scarb execute` command automatically
before proving the execution for you.

The proof for the trace files inside the execution folder will be generated, and a `proof.json` file will be placed inside the execution directory.

## Verifying Cairo proof

To verify the proof, you can run:

```shell
scarb verify <path to proof json file>
```

## Performance

The `stwo-cairo` prover can highly benefit from platform specific optimizations, that are not available when Scarb
is run from a precompiled binary.
For the best performance, it is recommended to build `scarb-prove` and `scarb-verify` crates from source,
with following compilation flags: `RUSTFLAGS="-C target-cpu=native -C opt-level=3" --features="std"`.
For production use, it is recommended to use the `stwo-cairo` prover directly.
