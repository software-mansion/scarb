# Testing Scarb projects

The `scarb test` command executes all unit and integration tests of a local package.
It is not a test runner by itself, but rather delegates work to a testing solution of choice.
Scarb comes with preinstalled `scarb cairo-test` extension, which bundles Cairo's native test runner.
It is the default test runner used by `scarb test`.

As for how to write Cairo tests, we recommend reading the "Testing Cairo Programs" chapter in the
[Cairo Programming Language](https://cairo-book.github.io/) book.

## Testing Starknet contracts

`scarb cairo-test` automatically enables Starknet-related testing features if the package depends on the
[`starknet`](./starknet/starknet-package) package.

## Using third-party test runners

The behaviour of the `scarb test` command can be changed by developers.
To do so, provide a script named explicitly `test` in the current workspace `Scarb.toml`.
If such script is found, Scarb will execute it instead of running the default test runner.

Scarb can be configured to use any tool in place of the default `cairo-test`, simply by changing
this configuration:

```toml filename="Scarb.toml"
[scripts]
test = "my-custom-command"
```

## Using Starknet Foundry

[Starknet Foundry](https://foundry-rs.github.io/starknet-foundry) is another project developed by [Software Mansion](https://swmansion.com/) team.
It enables advanced testing of [Starknet](https://www.starknet.io/) contracts, including fuzz testing, forking the network state and many more.

In order to tell `scarb test` to use Starknet Foundry testing in your project, type the following:

```toml filename="Scarb.toml"
[scripts]
test = "snforge test"
```

Do not forget to properly set up Starknet Foundry in your project beforehand.

## Using multiple test runners

The default test runner is regular Scarb extension, and thus it is always available directly, as `scarb cairo-test`
command.
With script-based override for the `scarb test` command, it is possible to perform arbitrary actions before and after
the test runner itself.
This trick also allows running multiple test runners in the project.
For example, to run a custom end-to-end test suite using a popular [pytest](https://pytest.org/),
[Starknet Devnet](https://0xspaceshard.github.io/starknet-devnet/) and [Starknet.py](https://starknetpy.rtfd.io/)
combination, type the following:

```toml
[scripts]
test = "scarb cairo-test && pytest"
```

It is also possible to run multiple Cairo test runners, if there is a sensible use case for this.
Mind, that you will probably want to conditionally disable tests depending on the running test runner, so that tests
will not be executed multiple times.
For example, to make `scarb test` run `scarb cairo-test` and then Starknet Foundry in a sequence, type the following:

```toml
[scripts]
test = "scarb cairo-test && snforge test"
```
