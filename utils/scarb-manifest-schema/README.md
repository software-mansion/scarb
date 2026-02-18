# scarb-manifest-schema

Small utility crate that exposes a dynamic JSON Schema for Scarb's `Scarb.toml` manifest, that maps to `scarb::core::TomlManifest`, and a helper to traverse it by a TOML path.

When to use it
- Build tooling that uses field descriptions, types, or examples of Scarb manifest keys.
- Docs generators that render Scarb manifest reference.

License
- Licensed under the same license as Scarb. See the repository's `LICENSE` file.
