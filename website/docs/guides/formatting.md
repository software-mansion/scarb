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

## Ignoring files

By default, Scarb will format all files with a `.cairo` extension from the directory containing the manifest file
and all the subdirectories.
If you want to ignore some paths while formatting, you can create a `.cairofmtignore` files with appropriate rules.
The format of these files is the same as the one used by `.gitignore` files.
You can create multiple ignore files on different levels of your directory structure, identically as you would do with
`.gitignore` files.

Additionally, files ignored by the `.gitignore` files will be omitted as well.
