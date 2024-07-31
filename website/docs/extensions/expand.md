# Expand

`scarb expand` is a tool that makes debugging macros in Scarb easier.

Before the actual compilation of your Cairo code, the Cairo compiler runs multiple pre-processing steps on it (these are
usually called plugins).
Each of these steps takes parsed Cairo code as an input, modifies it and returns modified Cairo code back to the
compiler.

This can often implement code generation, optimizations, or other transformations.
For instance, derives for Cairo structs can be implemented this way, or some boilerplate code can be generated.
This is also used in Cairo compiler to implement conditional compilation with `cfg` attributes.
Blocks of code under disabled `cfg` sets will be removed at this phase.

While preprocessing is useful for making the programmers code shorter and easier to reason about, it can also make
debugging harder.
Because of the preprocessing, Cairo code that is **actually** compiled can be different from the one you see in your
editor.

To help with debugging your code in such cases, you can use `scarb expand` command, which runs all preprocessing steps
on your package and return expanded Cairo code.
The expanded Cairo is saved as a file in your target directory.
