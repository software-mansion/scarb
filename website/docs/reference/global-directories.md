# Global Directories

Scarb uses several directories stored in operating system's conventional locations for different purposes, such as
storing global configuration or download and source cache.
This page list outlines all global directories, their default paths and information how to override them if possible.

## Cache directory

This is a location where Scarb will store a downloads, Git checkouts and package sources.

| Platform | Default Path                                    |
| -------- | ----------------------------------------------- |
| Linux    | `$XDG_CACHE_HOME/scarb` or `$HOME/.cache/scarb` |
| macOS    | `$HOME/Library/Caches/com.swmansion.scarb`      |
| Windows  | `%LocalAppData%\swmansion\scarb\cache`          |

This path can be overriden via `SCARB_CACHE` environment variable.

## Config directory

This is a location where Scarb will look for global configuration in the future.

| Platform | Default Path                                            |
| -------- | ------------------------------------------------------- |
| Linux    | `$XDG_CONFIG_HOME/scarb` or `$HOME/.config/scarb`       |
| macOS    | `$HOME/Library/Application Support/com.swmansion/scarb` |
| Windows  | `%LocalAppData%\swmansion\scarb\config`                 |

This path can be overriden via `SCARB_CONFIG` environment variable.

## Local data directory

This is a location, where users can put some additional data files for use by Scarb.
Scarb will look for [subcommands] in the `bin` subdirectory.

| Platform | Default Path                                            |
| -------- | ------------------------------------------------------- |
| Linux    | `$XDG_DATA_HOME/scarb` or `$HOME/.local/share/scarb`    |
| macOS    | `$HOME/Library/Application Support/com.swmansion.scarb` |
| Windows  | `%LocalAppData%\swmansion\scarb\data`                   |

This path cannot be overriden.

[subcommands]: ../writing-extensions/subcommands
