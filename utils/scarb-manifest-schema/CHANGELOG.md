# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

## 1.0.0 (2026-02-18)

- Add README and CHANGELOG documentation for the crate.

## 1.0.0 (2026-02-17)

- Initial release: introduce `scarb-manifest-schema` crate with:
  - `get_manifest_schema()` to generate the full manifest JSON Schema.
  - `SchemaTraverser` with `traverse` to resolve schema nodes by TOML path.
  - `get_shared_traverser()` providing a global, lazily initialized traverser.
