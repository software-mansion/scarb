use crate::support::CheckBuilder;
use indoc::formatdoc;

fn check(selector: &str, command: &str, output_type: &str, expected_output: &str) {
    CheckBuilder::default()
        .lib_cairo(formatdoc! {r#"
            #[executable]
            fn main() {{
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "shell:";
                connection_string.serialize(ref inputs);
                '{selector}'.serialize(ref inputs);
                let command: ByteArray = "{command}";
                command.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<{output_type}>(result);
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
fn taskeo_success() {
    check(
        "taskeo",
        "echo hello",
        "(i32, ByteArray)",
        "Result::Ok((0, \"hello\n\"))",
    )
}

#[test]
fn taskeo_failure() {
    check(
        "taskeo",
        "exit 1",
        "(i32, ByteArray)",
        "Result::Ok((1, \"\"))",
    )
}

#[test]
fn taskco_success() {
    check(
        "taskco",
        "echo hello",
        "ByteArray",
        "Result::Ok(\"hello\n\")",
    )
}

#[test]
fn taskco_failure() {
    check(
        "taskco",
        "exit 1",
        "ByteArray",
        "Result::Err(\"command failed with exit code: 1\")",
    )
}
