# Using cairo-run in Scarb projects

The `scarb cairo-run` command executes the `main` function of a local package.
It does not compile any cairo code within the package so using `scarb build` beforehand is necessary.
There are two additional optional parameters: `--available-gas` and `--print-full-memory` which can be used
if necessary.
