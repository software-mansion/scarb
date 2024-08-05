# Conditional Compilation

_Conditionally compiled source code_ is source code that may or may not be considered a part of the source code
depending on certain conditions.
Source code can be conditionally compiled using the [`cfg`](#the-cfg-attribute) attribute.
These conditions are based on the target environment of the compiled package and a few other miscellaneous things
further described below in detail.

_Configuration options_ are names and key-value pairs that are either set or unset.
Names are written as a single identifier such as, for example, `test`.
Key-value pairs are written as a named function argument: an identifier, `:`, and then a short string.
For example, `target: 'starknet-contract'` is a configuration option.

Keys are not unique in the set of key-value configuration options.
For example, both `opt: 'x'` and `opt: 'y'` can be set at the same time.

## Forms of conditional compilation

### The `cfg` attribute

The `cfg` attribute conditionally includes the thing it is attached to based on a configuration predicate. It is written
as `#[cfg(configuration predicate)]`. If the predicate is true, the item is rewritten to not have the `cfg` attribute on
it. If the predicate is false, the item is removed from the source code.

For example, this attribute can be used to provide different implementations of a function depending on current
Scarb [target](./targets):

```cairo
#[cfg(target: 'lib')]
fn example() -> felt252 {
    42
}

#[cfg(target: 'starknet-contract')]
fn example() -> felt252 {
    512
}
```

## Set configuration options

Which configuration options are set is determined statically during the compilation of the compilation unit of the
compiled package.
It is not possible to set a configuration option from within the source code of the package being compiled.

### `target`

Key-value option set once with the current compilation unit's [target](./targets).

Example values:

- `'lib'`
- `'starknet-contract'`
