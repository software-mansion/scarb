use crate::support::CheckBuilder;
use indoc::formatdoc;

#[test]
fn exec_success() {
    CheckBuilder::default()
        .lib_cairo(formatdoc! {r#"
            #[executable]
            fn main() {{
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "shell:";
                connection_string.serialize(ref inputs);
                'exec'.serialize(ref inputs);
                let command: ByteArray = "echo hello";
                command.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<(i32, ByteArray)>(result);
            }}
        "#})
        .stdout_matches(formatdoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Result::Ok((0, "hello
            "))
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}

#[test]
fn exec_failure() {
    CheckBuilder::default()
        .lib_cairo(formatdoc! {r#"
            #[executable]
            fn main() {{
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "shell:";
                connection_string.serialize(ref inputs);
                'exec'.serialize(ref inputs);
                let command: ByteArray = "exit 1";
                command.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<(i32, ByteArray)>(result);
            }}
        "#})
        .stdout_matches(formatdoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Result::Ok((1, ""))
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}

#[test]
fn exec_nonexistent_command() {
    CheckBuilder::default()
        .lib_cairo(formatdoc! {r#"
            #[executable]
            fn main() {{
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "shell:";
                connection_string.serialize(ref inputs);
                'exec'.serialize(ref inputs);
                let command: ByteArray = "nonexistent_command_123";
                command.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<(i32, ByteArray)>(result);
            }}
        "#})
        .stdout_matches(formatdoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Result::Ok((127, ""))
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}
