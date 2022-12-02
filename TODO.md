# TODO

## Marek

1. `murek metadata`, 1:1 how it's done in Cargo
2. UI
3. PubGrub
4. StarkNet: `[command-dependencies]` and `[[command.provides.build.step.post]]`?

## Free to take

* Hide `Summary` behind `Arc` like `Package` is to reduce clone costs.
* Parallel downloads, use `futures::join_iter`. Should be trivial few lines change.
* Pass `PATH` and `MUREK_LOG` to external commands
* CI
* `list-commands`. Define an op `ops::subcommand` which does the same as `list_commands` in Cargo's main.
* Package metadata (authors, descriptions, links etc.. copy-paste from Cargo)
* Pick TODO comments from codebase
* `murek fmt`, again, as an extension.
* `add` & `rm`, take a look at `toml_edit` crate and how it's done in Cargo
* `GitSource` almost 1:1 how it's done in Cargo
* Make workspaces really handle multiple packages
* Test runner. Add new workspace member `murek-test` and develop this as a subcommand!
