use crate::support::CheckBuilder;
use indoc::formatdoc;

fn check(command: &str, expected_output: &str) {
    CheckBuilder::default()
        .lib_cairo(formatdoc! {r#"
            #[executable]
            fn main() {{
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "shell:";
                connection_string.serialize(ref inputs);
                let selector: ByteArray = "exec";
                selector.serialize(ref inputs);
                let command: ByteArray = "{command}";
                command.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<(i32, ByteArray)>(result);
            }}
        "#})
        .stdout_matches(formatdoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            {expected_output}
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}

#[test]
fn exec_success() {
    check("echo hello", "Result::Ok((0, \"hello\n\"))")
}

#[test]
fn exec_failure() {
    check("exit 1", "Result::Ok((1, \"\"))")
}

#[test]
fn exec_nonexistent_command() {
    check("nonexistent_command_123", "Result::Ok((127, \"\"))")
}
