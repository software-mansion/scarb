# `scarb-oracle-hint-service`

Oracle hint service from [Scarb](https://docs.swmansion.com/scarb).

This crate provides oracle functionality for Cairo programs as executed by Scarb.
It handles oracle hints and manages connections to external oracle services through various protocols.
Use it in your custom executors if you want your executor to have feature parity with `scarb execute`, `cairo-test`
and `snforge`.
