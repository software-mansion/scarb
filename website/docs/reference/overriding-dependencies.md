# Overriding Dependencies

Overriding dependencies is a powerful feature in Scarb that allows you to replace all usages of some dependency with
another revision, coming from a different source, during the workspace resolution.

The desire to override a dependency can arise through a number of scenarios.
Most of them, however, boil down to the ability to work with a package before it’s been published to
[scarbs.xyz](https://scarbs.xyz/) registry.
For instance, imagine you depend on some upstream package from the registry, which you don't control.
This package can transiently depend on another package that has a bug fix that has been merged to its git repository
but has not yet been published to the registry.
If you want to force your build to use the fixed version of the package, you can set an override for it in your manifest.
This way you can use it without the need to publish the upstream package to the registry or modify your dependency
manifest.

## The `[patch]` section

You can specify overrides for packages in your dependency tree in the `[patch]` section of your workspace root manifest.

> [!NOTE]
> The `[patch]` section can only be defined in the workspace root manifest.
> If you specify it in a workspace member manifest, Scarb will exit with an error.

The `[patch]` section consists of a set of sub-tables, identified by the source URL of the dependency you want to override.
Each of the sub-tables can be defined with syntax identical as `[dependency]` section.
To override the default registry, you can use the `scarbs-xyz` identifier, without the need to provide full URL.
See [specifying dependencies](./specifying-dependencies.md) for more details of the syntax.

Examples:

```toml
# Override a registry dependency with a local path
[patch.scarbs-xyz]
foo = { path = "/path/to/foo" }

# Override a git dependency with a registry version
[patch."https://github.com/example/foo"]
foo = "1.0.0"

# Override a path dependency with a registry version
[patch."file:/path/to/foo/Scarb.toml"]
foo = "1.0.0"
```
