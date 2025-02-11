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

Alternatively, you can use the `--emit stdout` argument.
With this argument, Scarb will not make any changes to the files on your disk.
Instead, full new content of formatted files will be printed to stdout, prepended with their path.
Files that had already been in a correct format will not be emitted.
This may be useful for integrating with some external tools.

You can choose packages to format with `--package / --workspace` arguments.
When formatting a package, all cairo files in package root and directories below will be formatted (not only the `src/*`
directory).

## Formatting options

You can add `[tool.fmt]` section inside `Scarb.toml` to override the default formatter configuration.

```toml
[tool.fmt]
sort-module-level-items = true
```

### Available configuration option

- `sort-module-level-items`

  Reorder import statements alphabetically in groups (a group is separated by a newline).\
  **Default:** `true`

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

## Ignoring lines

Mark statements or expressions with `#[cairofmt::skip]` attribute to omit them during formatting. See example below:

```cairo
#[cairofmt::skip]
let a = array![
    1,
    2,
    3
    ];
```
