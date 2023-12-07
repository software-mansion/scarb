# Using cairo-run in Scarb projects

The `scarb cairo-run` command executes the `main` function of a local package.
It does automatically compile the cairo code within the package so using `scarb build` beforehand is not necessary.
This automatically called build can be skipped with the `--no-build` flag.
The extension accepts two optional parameters: `--available-gas` and `--print-full-memory`.
The first can be used to set the available gas for the execution and the second prints the full memory after the execution.
