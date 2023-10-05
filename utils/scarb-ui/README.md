# `scarb-ui`

Terminal user interface primitives used by [Scarb](https://docs.swmansion.com/scarb) and its extensions.

This crate focuses mainly on two areas:

1. Serving a unified interface for communication with the user, either via:
    - rendering human-readable messages or interactive widgets,
    - or printing machine-parseable JSON-NL messages, depending on runtime configuration.
2. Providing reusable [`clap`](https://crates.io/crates/clap) arguments for common tasks.

See [crate documentation](https://docs.rs/scarb-ui) for more information.
