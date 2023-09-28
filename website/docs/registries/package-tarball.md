# Package tarball

Package tarballs are a distributable, compressed `.tar.zst` files with the source code of the package and additional
metadata for use by registries and other services.
Tarballs are regular [GNU tar archives](<https://en.wikipedia.org/wiki/Tar_(computing)>) compressed
with [Zstandard](https://facebook.github.io/zstd/) algorithm.
The `scarb package` command can be used to create a package tarball from a package directory.

In general a package tarball consists minimum amount of files copied from package source directory and several
additional metadata files.
Scarb does not permit source files named like metadata files (case-insensitive) to be included in the tarball.

## Metadata

The package tarball contains the following metadata files:

### `VERSION`

The tarball version as a single ASCII integer.
The current tarball version is `1`.

### `Scarb.toml`

The package's `Scarb.toml` rewritten and normalized, so that contains only the most important information about a
package in order to be built, processed in version resolution algorithm and presented in the registry.

The normalization process consists of the following:

1. All workspace references are expanded.
2. All dependency specifications are stripped from non-registry source properties. For example:

   ```toml
   [dependencies]
   foobar = { version = "1.2.3", path = "../foobar" }
   ```

   is reduced to:

   ```toml
   [dependencies.foobar]
   version = "1.2.3"
   ```

3. All sections other than `[package]`, `[dependencies]` and `[tool]` are removed from the manifest.
4. All auto-detected properties, like `package.readme` are explicitly stated.

### `Scarb.orig.toml`

The original `Scarb.toml` file from the package source directory, without any processing.

## Package source

By default, only the `src` directory from package source is included in the tarball.
Additionally, the readme and license files may be included, if relevant fields are present in the source `Scarb.toml`
file (or their values were auto-detected).
