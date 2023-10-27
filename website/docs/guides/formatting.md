# Formatting

Scarb comes with a built-in Cairo source code formatter:

```shell
scarb fmt
```

If you use continuous integration workflows in your project, you can also add a step to ensure the Cairo code is
properly formatted:

```shell
scarb fmt --check
```

## Formatting options

You can add `[tool.fmt]` section inside `Scarb.toml` to override the default formatter configuration.

```toml
[tool.fmt]
sort-module-level-items = true
```

### Available configuration option

- `sort-module-level-items`

  Reorder import statements alphabetically in groups (a group is separated by a newline).\
  **Default:** `false`

- `max-line-length`

  Maximum width of each line.\
  **Default:** `100`

- `tab-size`

  Number of spaces per tab.\
  **Default:** `4`

## Ignoring files

By default, Scarb will format all files with a `.cairo` extension from the directory containing the manifest file
and all the subdirectories.
If you want to ignore some paths while formatting, you can create a `.cairofmtignore` files with appropriate rules.
The format of these files is the same as the one used by `.gitignore` files.
You can create multiple ignore files on different levels of your directory structure, identically as you would do with
`.gitignore` files.

Additionally, files ignored by the `.gitignore` files will be omitted as well.
