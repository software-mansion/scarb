# Custom subcommands

Scarb is designed to be extensible with new subcommands without having to modify Scarb itself.
This is achieved by translating a Scarb invocation of the form `scarb (?<command>[^ ]+)` into an invocation of an
external tool `scarb-${command}`.
The external tool must be present in (preferably) the `bin` directory in Scarb's [local data directory][dirs], or in any
other of the user's `$PATH` directories.

When Scarb invokes a custom subcommand, the first argument to the subcommand will be the filename of the custom
subcommand, as usual.
The second argument will be the subcommand name itself.
For example, the second argument would be `${command}` when invoking `scarb-${command}`.
Any additional arguments on the command line after `${command}` will be forwarded unchanged.

## Environment variables

Additionally, Scarb passes more contextual information via environment variables:

| Environment variable  | Description                                                                                         |
|-----------------------|-----------------------------------------------------------------------------------------------------|
| `SCARB`               | Path to Scarb executable.                                                                           |
| `PATH`                | System `$PATH` but augmented with `bin` directory in Scarb's [local data directory][dirs].          |
| `SCARB_CACHE`         | Path to Scarb's [cache][dirs] directory.                                                            |
| `SCARB_CONFIG`        | Path to Scarb's [config][dirs] directory.                                                           |
| `SCARB_TARGET_DIR`    | Path to the current target directory.                                                               |
| `SCARB_PROFILE`       | Name of the current profile.                                                                        |
| `SCARB_MANIFEST_PATH` | Absolute path to current `Scarb.toml`.                                                              |
| `SCARB_UI_VERBOSITY`  | Scarb's messages verbosity, possible values: `quiet`, `normal`, `verbose`.                          |
| `SCARB_LOG`           | Scarb's logger directives, follows [`tracing`'s `EnvFilter` syntax][tracing-env-filter].            |
| `SCARB_TEST_RUNNER`   | Test runner to use when calling `new` or `init`. possible values: `starknet-foundry`, `cairo-test`. |

## Implementation recommendations

Custom subcommands may use the `SCARB` environment variable to call back to Scarb.
The [`scarb metadata`](./scarb-metadata) command can be used to obtain information about the current project,
whereas the [`--json`](./json-output) flag make Scarb output machine-readable messages on standard output.
If you are using Rust, the [`scarb-metadata` crate](https://crates.io/crates/scarb-metadata) can be used to parse the
output.

[dirs]: ../reference/global-directories
[tracing-env-filter]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives
