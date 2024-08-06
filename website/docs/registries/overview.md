# Overview

Scarb is not only a Cairo build toolchain but also a package manager.
It uses registries to store and manage packages.
While Scarb supports alternative ways to host packages, registries make it easy to publish, discover, and integrate packages into your projects.

## Official registry

Currently, the default way to host packages is via the official [scarbs.xyz](https://scarbs.xyz) registry.
Please note that the official registry is still in development.
You can already use it to discover and [add](#adding-dependencies) packages to your projects.
[Publishing](#packaging-and-publishing) packages is currently limited, but if there are any other packages you would like to be available - please reach out to us on [Telegram](https://t.me/scarbs_xyz) or [Discord](https://discord.gg/7YXj4Z2).

## Adding dependencies

If you want to add a package from the official registry as a dependency, you can read about it [here](./../reference/specifying-dependencies#specifying-dependencies-from-official-registry).

## Packaging and publishing

If you are interested in learning about the packaging and publishing process, you can read about it [here](./packaging).

## Custom registry

Although Scarb uses the official registry by default,
you can [host your own](./custom-registry) registry or search for and [use](./custom-registry#using-custom-registry) a community-hosted one instead.
