# scarb-proc-macro-host

The Cairo compiler plugin that drives procedural macro expansion.

This crate contains the logic that sits between the Cairo compiler and a procedural macro
implementation: finding which macros apply to an AST item, building the input token stream,
adapting token spans, and mapping the expansion output back onto the original source.

It is generic over a [`ProcMacroBackend`], which supplies the list of available macro expansions
and performs the actual expansion. Scarb implements that backend by calling into procedural macro
dynamic libraries loaded in-process. CairoLS implements it by talking to
`scarb proc-macro-server`.
