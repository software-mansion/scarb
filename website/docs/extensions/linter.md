# Cairo lint

`scarb lint` is a tool that run checks and fixes (if possible and specified) various code mistakes. 

## Additional Documentation

For those who want to explore the linter much deeper, we suggest visiting [cairo-lint](https://github.com/software-mansion/cairo-lint) repository, as it's the one that Scarb uses under the hood.

## Basic Arguments

- `--test` - Should lint the tests.
- `--fix` - Should fix the lint when it can.
- `--ignore-cairo-version` - Do not error on `cairo-version` mismatch.
