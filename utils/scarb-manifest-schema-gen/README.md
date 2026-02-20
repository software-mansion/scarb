# scarb-manifest-schema-gen

An internal workspace utility to synchronise the scarb-manifest-schema JSON snapshot with the source of truth in the scarb core crate.

## Purpose
The `scarb-manifest-schema` crate provides a JSON Schema for Scarb's `Scarb.toml` manifest. 
To avoid making the core scarb crate a mandatory runtime dependency for the `scarb-manifest-schema`, we use a snapshotting strategy:

- Source: The `TomlManifest` struct lives in the scarb `scarb::core::manifest` crate.

- This Tool: Generates a `schema.json` file from the Rust structs.

- Library: The `scarb-manifest-schema` exposes the schema to use outside the Scarb workspace.

## Usage
You should run this tool whenever you modify the `TomlManifest` struct in `/scarb/src/core/manifest/toml_manifest.rs`

To update run: 
```bash
cargo run -p scarb-manifest-schema-gen
```

## Consistency Check
The `test_schema_is_up_to_date` test in `scarb-manifest-schema` ensures that the snapshot is up to date.
If you modify the manifest but forget to run this tool, the consistency test in scarb-manifest-schema will fail in the CI/CD pipeline, reminding you to run this tool locally and commit the changes.

To verify everything is in sync, you can run:
```bash
cargo test -p scarb-manifest-schema
```
