# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

## 0.7.0
- Expansion results now carry a `SpannedTokenStream` (a list of tokens, each with the span of the
  code it came from) instead of a flat string plus optional code mappings. Callers compute code mappings from the token spans themselves.
  Expansions performed through the v1 procedural macro api, which has no spans, are reported as a
  single token covering the whole expansion origin, so callers no longer need to special-case
  them.
- Remove `ProcMacroResult::code_mappings`, along with the `CodeMapping` and `CodeOrigin` types.
- Remove the `conversions` module. Converting between macro api versions is now entirely internal
  to the proc macro server.
- `ExpandDerive` responds with one `ProcMacroResult` per requested derive, instead of a single
  result holding the concatenated code of all of them. Results come in the order the derives were
  requested in, and the server no longer sorts them by name.
- Each result's `fingerprint` is now the fingerprint of the macro that produced that one derive,
  instead of a hash combining all derives of the item.
- Callers concatenate the per-derive expansions themselves, offsetting code mappings as they go.
- Drop the dependency on `cairo-lang-macro` 0.1.
- Add `MacroWithHash::cairo_name`, the name the macro is written under in Cairo code. `name` stays
  the name of the expansion function, used when requesting expansions.

## 0.6.0 (2026-09-23)
- Pass a call site per derive in `ExpandDeriveParams` (`derives: Vec<Derive>`), replacing the single `call_site`.

## 0.5.0 (2025-12-10)
- Add `MacroWithHash` struct. 
- Use `MacroWithHash` to identify procedural macros in `CompilationUnitComponentMacros`, instead of macro name only.
- Add `fingerprint` field to `ProcMacroResult` struct.

## 0.4.0 (2025-10-23)
- Add `Workspace` type.
- Limit `DefinedMacros` request and `ProcMacroScope` to specific Scarb workspace.

## 0.3.0 (2025-08-04)
- Promote `0.3.0-rc.1` to stable release.

## 0.3.0-rc.1 (2025-07-10)
- Pass `adapted_call_site` in expand attributes.
- Disallow creating cairo_lang_macro::Diagnostic without the constructor.

## 0.3.0-rc.0 (2025-06-06)
- Support `cairo-lang-macro 0.2.0-rc.0` and new procedural macro API.

## 0.2.0 (2025-02-25)
- Respect crate scoped cairo plugins.

## 0.1.0 (2024-11-13)
- Initial release.
