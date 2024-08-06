# Publishing your package

To share your package, it needs to be packaged into an archive and uploaded to the registry.
Once uploaded, it will be available for other users to download.

## Packaging the package

Use the `scarb package` command to create an archive of your package.
You can read about the package compression algorithm and contents in the [Package tarball](./package-tarball) section.
Basically when you run the command, Scarb gathers the source code of your package along with metadata files, such as the manifest file, and places them in an archive in `target/package` directory.

If you are in a Git repository, Scarb will first check if the repo state is clean and error out in case of any changes present in the Git working directory.
To bypass this check, you can use the `--allow-dirty` flag.

The next step is package verification.
After creating the initial archive, Scarb will attempt to unpack it and compile to check for any corruptions in the packaging process.
If you want to speed up the packaging process, you can disable this step using the `--no-verify` flag.

> [!WARNING]
> This is a dangerous operation as it can lead to uploading a corrupted package to the registry.
> Please use with caution.

After successfully completing the whole process, the `{name}-{version}.tar.zst` archive waits in the `target/package` directory for being uploaded, where both `name` and `version` correspond to the values in `Scarb.toml`.

## Publishing the package

To upload your package, you can use the `scarb publish` command.
Publishing your package over HTTP is not yet supported, therefore, the only way to upload the package is to use a local registry.
The command takes the `--index` argument that you can use to pass the local directory path where you want to store the packages.

```shell
scarb publish --index file:///Users/foo/bar
```

This is only useful when you are [hosting your own registry](./custom-registry).
