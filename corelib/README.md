This directory contains vendored Cairo [`core`], which is then embedded into Scarb.
It is important to keep this checkout synchronized with used Cairoc compiler version.
In the future we plan to automatically pull `core` from Cairo's repository.

This directory also contains a custom [`Scarb.toml`](./Scarb.toml) for this package.

[`core`]: https://github.com/starkware-libs/cairo/tree/v1.0.0-alpha.3/corelib
