# Packaging your project

When you want to share your package over package registry, it needs to be turned into an archive first. The archive will then be uploaded to the registry and downloaded by other users.

Use `scarb package` command to create an archive of your package. You can read about package compression algorithm and contents in [Package tarball](/docs/registries/package-tarball) section.
Basically when you run the command Scarb looks for the source code of your package together with metadata files such as manifest file and put them in an archive in `target/package` directory.

If you are in a Git repository, Scarb will first check if the repo state is clean and error out in case of any changes present in a git working directory. To ignore this check you can use the `--allow-dirty` flag.

Next step is the package verification. After creating an initial archive, Scarb will attempt to unpack it and compile to check for any corruptions in the packaging process. If you want to speed up the package process you can disable this step using the `--no-verify` flag.

After successfully passing the whole process, the `{name}-{version}.tar.zst` archive waits in the `target/package` directory for being uploaded, where both `name` and `version` correspond to the values in `Scarb.toml`.

## Publishing the package

To upload your package, you can use the `scarb publish` command. Publishing your package over HTTP is not yet supported, therefore, the only way to upload the package is to use local registry. The command takes `--index` argument that you can use to pass local directory path where you want to store the packages.

```shell
scarb publish --index file:///Users/foo/bar
```

This is only useful when you are [hosting your own registry](/docs/registries/custom-registry).
