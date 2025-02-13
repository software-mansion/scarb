# Cairo lint

`scarb lint` is a static code analysis tool for the Cairo language.

It can help you improve your code quality and consistency by checking the codebase against a set of predefined rules, called lints.
It can also automatically fix some of the issues found.

## Getting started

To run `lint` in the current project, just type:

```sh
scarb lint
```

This will run the code analysis and suggest places to edit your code.
Running `lint` will yield issues like this:

```sh
$ scarb lint
  Linting hello_world v0.1.0 (/hello_world/Scarb.toml)
  warning: Plugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.
   --> /hello_world/src/lib.cairo:2:8
    |
  2 |     if is_true() == true {
    |        -----------------
    |
```

To attempt to fix the issues automatically, you can run:

```sh
scarb lint --fix
```

You can also specify `--test` to perform analysis of your project's tests as well (i.e. all the Cairo code under `#[cfg(test)]` attributes).
To learn more about available arguments, just run `scarb lint --help`.

## Learning more

For those who want to explore the linter much deeper, we suggest visiting [cairo-lint](https://github.com/software-mansion/cairo-lint) repository, as it's the one that Scarb uses under the hood.
