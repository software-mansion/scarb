# Testing Scarb projects

The `scarb test` command executes all unit and integration tests of a local package.
It is not a test runner by itself, but rather delegates work to a testing solution of choice.
Scarb comes with preinstalled `scarb cairo-test` extension, which bundles Cairo's native test runner.
It is the default test runner used by `scarb test`.

The `scarb cairo-test` extension calls `scarb build --test` to build actual executable test files,
using the [test targets](../reference/targets#test-targets) mechanism.
The extension itself only relies on produced artifacts to actually run the tests.

As for how to write Cairo tests, we recommend reading the "Testing Cairo Programs" chapter in the
[Cairo Programming Language](https://book.cairo-lang.org/) book.

## Testing Starknet contracts

`scarb cairo-test` automatically enables Starknet-related testing features if the package depends on the
[`starknet`](./starknet/starknet-package) package.

## Tests organization

Scarb supports two types of tests: unit and integration.
Unit tests are defined in the main package file, while integration tests are defined in separate files in
directory called `tests` besides the manifest file.
From integration tests, you can only reference the main package by package name (as if you would add it as dependency).
The integration tests can be either a single module with a `lib.cairo` file in `tests` directory,
or multiple files with `cairo` extension, each defining a separate test module.

> [!NOTE]
> For now, the compilation of integration tests with `lib.cairo` file in the `tests` directory will be faster than
> compilation of integration tests defined in separate files.

## Using third-party test runners

The behaviour of the `scarb test` command can be changed by developers.
To do so, provide a script named explicitly `test` in the current workspace `Scarb.toml`.
If such script is found, Scarb will execute it instead of running the default test runner.

Scarb can be configured to use any tool in place of the default `cairo-test`, simply by providing
a custom script named `test`:

```toml filename="Scarb.toml"
[scripts]
test = "command-to-run-tests"
```

## Using Starknet Foundry

[Starknet Foundry](https://foundry-rs.github.io/starknet-foundry), like Scarb, is a project developed
by [Software Mansion](https://swmansion.com/) team.
It enables advanced testing of [Starknet](https://www.starknet.io/) contracts, including fuzz testing, forking the
network state, setting-up a specific contract state in your tests, and many more.

In order to tell `scarb test` to use Starknet Foundry as the test runner testing in your project, define the following:

```toml filename="Scarb.toml"
[scripts]
test = "snforge test"
```

Do not forget to
properly [set up Starknet Foundry in your project](https://foundry-rs.github.io/starknet-foundry/getting-started/first-steps.html#using-snforge-with-existing-scarb-projects)
beforehand.

## Using multiple test runners

The default test runner is regular Scarb extension, and thus it is always available directly, as `scarb cairo-test`
command.
With script-based override for the `scarb test` command, it is possible to perform arbitrary actions before and after
the test runner itself.
This trick also allows running multiple test runners in the project.
For example, to run a custom test suite using [pytest](https://pytest.org/) after the `cairo-test` one, type the
following:

```toml
[scripts]
test = "scarb cairo-test && pytest"
```
