# Incremental Compilation

## Macros and incremental compilation: invalidating caches with fingerprints

Scarb implements incremental caching, which means that subsequent builds can be sped up with use of caches produced
during former builds.

This is possible because the relation between Cairo code and produced artifacts is **deterministic**.
During the compilation we can save some state of the compiler at some point in time and then load it in another run
from disk and continue, as if we never stopped compiling.

As procedural macros can inject additional logic defined by the macro author, it needs to uphold the same determinism
assumptions as the compiler itself.

> [!WARNING]
> This means that **all macro outputs** should be **deterministic** in regard to **the macro input passed by Scarb**
> (i.e. the token stream the macro implementation receives as an argument).

If your macro needs to read inputs from other sources that Scarb is not aware of, say from environmental variables,
you need to define a _fingerprint_ for this input with [the fingerprint attribute](https://docs.rs/cairo-lang-macro/latest/cairo_lang_macro/attr.fingerprint.html)
from procedural macro API.
Fingerprint is a function that returns a single `u64` value.
If the value changes, Scarb will invalidate incremental caches for code depending on this macro.
This enables the macro author to manually invalidate caches based on external inputs.
Usually, this is simply a hash of the input (note that you need to use a stable hash function, like `xxh3`, not
rng-seeded ones, like the default hasher used in Rust).
