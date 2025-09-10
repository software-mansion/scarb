use crate::support::CheckBuilder;
use indoc::indoc;

/// This tests checks two things at once:
/// 1. That we react to binaries that don't talk JSON-RPC.
/// 2. That we look for binaries in $PATH.
#[test]
fn non_jsonrpc_command_from_path() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "stdio:whoami";
                connection_string.serialize(ref inputs);
                'pow'.serialize(ref inputs);
                (4).serialize(ref inputs);
                (2).serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<Span<felt252>>(result);
            }
        "#})
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Result::Err("oracle process is misbehaving: expected JSON-RPC message starting with '{', got byte: '[..]'")
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}

#[test]
fn non_existent_file() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "stdio:i_definitelly_do_not_exist.exe";
                connection_string.serialize(ref inputs);
                'hello'.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<Span<felt252>>(result);
            }
        "#})
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Result::Err("failed to spawn oracle process

            Caused by:
                [..]")
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}

#[test]
fn non_executable_file() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "stdio:Scarb.toml";
                connection_string.serialize(ref inputs);
                'hello'.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<Span<felt252>>(result);
            }
        "#})
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Result::Err("failed to spawn oracle process

            Caused by:
                [..]")
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}

/// If the `./ ` prefix is omitted from the path, then the oracle should be looked for in $PATH,
/// which doesn't have `.` in it.
#[test]
fn test_oracle_without_dot_slash() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "stdio:test_oracle.py";
                connection_string.serialize(ref inputs);
                'panic'.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<Span<felt252>>(result);
            }
        "#})
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Result::Err("failed to spawn oracle process

            Caused by:
                [..]")
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}

#[test]
fn test_oracle() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {
                let connection_string: ByteArray = "stdio:python3 ./test_oracle.py";

                let mut inputs: Array<felt252> = array![];
                connection_string.serialize(ref inputs);
                'sqrt'.serialize(ref inputs);
                (16).serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<felt252>(result);

                let mut inputs: Array<felt252> = array![];
                connection_string.serialize(ref inputs);
                'panic'.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<felt252>(result);
            }
        "#})
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Result::Ok(4)
            Result::Err("oops")
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}
