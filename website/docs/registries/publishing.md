# Publishing your package

To share your package, it must be packaged into an archive and uploaded to the registry.
Once uploaded, it will be available for other users to download and use.

## Publishing the package

To upload your package, use the scarb publish command.
By default, this command will publish your package to the official [scarbs.xyz](https://scarbs.xyz) registry.
The publish command automatically [packages and verifies](#packaging-your-package) your package, so there is no need to run `scarb package` beforehand.

To publish your package to a registry that supports package publishing, you need to authenticate using an API token with the `publish` scope.
First, log in to the registry and [in the dashboard](https://scarbs.xyz/dashboard) generate the API token.
Scarb will use the token to authenticate and complete the publishing process.
The token must be provided via the `SCARB_REGISTRY_AUTH_TOKEN` environment variable.

```shell
SCARB_REGISTRY_AUTH_TOKEN=scrb_mytoken scarb publish
```

> [!NOTE]
> In case of any problems with publishing of your package to the registry
> please reach out to us on [Telegram](https://t.me/scarbs_xyz) or [Discord](https://discord.gg/7YXj4Z2).

### Publishing to a custom registrty

You can also publish your package to a custom registry by using the --index argument.
This allows you to specify the path to a local directory where you want to store your packages.

```shell
scarb publish --index file:///Users/foo/bar
```

## Preventing package from being published

If you want to prevent your package from being published, you can add the `publish = false` in `Scarb.toml`.

```toml
[package]
publish = false
```

## Packaging your package

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
