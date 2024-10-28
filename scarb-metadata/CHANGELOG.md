# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

## 1.13.0 (2024-10-28)
- Add `CompilationUnitComponentId`.
- Add `id` field on `CompilationUnitComponentMetadata`.
- Add `CompilationUnitComponentDependencyMetadata`.
- Add `dependencies` field to `CompilationUnitComponentMetadata`. 
- Add `discriminator` field to `CompilationUnitComponentMetadata`

## 1.12.0 (2024-04-09)
- Added `cfg` field on `CompilationUnitComponentMetadata`.

## 1.11.1 (2024-02-06)
- Fix backward compatibility of `experimental_features` field deserialization.
- Added `profile`, `dev` and `release` arguments to `MetadataCommand`.

## 1.11.0 (2024-01-31)
- Added `experimental_features` field to `PackageMetadata`.
- Added `inherit_stdout` and `json` flags to `MetadataCommand`. Scarb output is now textual by default.   

## 1.10.0 (2023-12-13)
- Added `kind` field to `DependencyMetadata`.

## 1.9.0 (2023-11-09)
- Added `edition` field to `PackageMetadata`.

## 1.8.0 (2023-09-25)
- **Removed** `packages_filter` feature from `scarb-metadata`. This change is technically breaking, but we did not detect any usage of this feature in the wild.

## 1.7.1 (2023-09-18)
- Fix `runtime_manifest` field not working with Scarb `<0.5.0`.

## 1.7.0 (2023-08-23)
- **Deprecated** `packages_filter` feature in `scarb-metadata`. **NOTE:** Use `scarb-ui` from now on.

## 1.6.0 (2023-08-11)
- Added `ScarbCommand` abstraction.

## 1.5.0 (2023-08-07)
- Added `--workspace` flag to `PackagesFilter`.
- Removed `rust-version` requirement.

## 1.4.2 (2023-05-29)
- Fixed deserialization of metadata from Scarb `<0.3`.

## 1.4.1 (2023-05-29)
- Show `packages_filter` feature docs on [docs.rs](https://docs.rs).

## 1.4.0 (2023-05-29)
- **Removed** `builder` from default features set.
- Added `cairo_plugins` to compilation unit metadata.
- Added `PackagesFilter` feature.
- Added `extra` field capturing additional data in metadata structs
- Added non-panicking getters for package and compilation unit metadata

## 1.3.0 (2023-05-17)
- **Removed** `CompilationUnitMetadata.components_legacy` field.

## 1.2.0 (2023-05-02)
- Added cfg items to compilation unit metadata.

## 1.1.0 (2023-04-13)
- Added profiles support.

## 1.0.1 (2023-04-13)
- Many small fixes and additions to initial release.

## 1.0.0 (2023-03-17)
- Initial release.
