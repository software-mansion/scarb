//! These tests rely on prebuilt wasm fixtures committed to the repository.
//! To regenerate them, run the build-fixtures.sh script.

use crate::support::CheckBuilder;
use assert_fs::prelude::*;
use indoc::indoc;
use predicates::prelude::*;

#[test]
fn wasip2() {
    let t = CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {{
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "wasm:wasip2.wasm";
                connection_string.serialize(ref inputs);
                let selector: ByteArray = "add";
                selector.serialize(ref inputs);
                (1_u64).serialize(ref inputs);
                (2_u64).serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<u64>(result);

                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "wasm:wasip2.wasm";
                connection_string.serialize(ref inputs);
                let selector: ByteArray = "join";
                selector.serialize(ref inputs);
                let a: ByteArray = "foo";
                a.serialize(ref inputs);
                let b: ByteArray = "bar";
                b.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<ByteArray>(result);

                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "wasm:wasip2.wasm";
                connection_string.serialize(ref inputs);
                let selector: ByteArray = "io";
                selector.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<()>(result);

                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "wasm:wasip2.wasm";
                connection_string.serialize(ref inputs);
                let selector: ByteArray = "count";
                selector.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<u64>(result);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<u64>(result);

                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "wasm:wasip2.wasm";
                connection_string.serialize(ref inputs);
                let selector: ByteArray = "fs";
                selector.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<Result<ByteArray, ByteArray>>(result);

                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "wasm:wasip2.wasm";
                connection_string.serialize(ref inputs);
                let selector: ByteArray = "network";
                selector.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<Result<ByteArray, ByteArray>>(result);
            }}
        "#})
        .asset("wasip2.wasm", include_bytes!("wasip2.wasm"))
        .dir_op(|t| {
            t.child("read_file.txt")
                .write_str("hello from the outside")
                .unwrap()
        })
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Result::Ok(3)
            Result::Ok("foobar")
            stdout is working as expected
            Result::Ok(())
            Result::Ok(0)
            Result::Ok(1)
            Result::Ok(Result::Ok("hello from the outside"))
            Result::Ok(Result::Ok("tcp connectivity works"))
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .stderr_contains("stderr is working as expected\n")
        .check();

    t.child("write_file.txt")
        .assert(predicate::eq("hello from wasm"));
}

#[test]
fn naked() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "wasm:naked.wasm";
                connection_string.serialize(ref inputs);
                let selector: ByteArray = "add";
                selector.serialize(ref inputs);
                (1_i64).serialize(ref inputs);
                (2_i64).serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<i64>(result);

                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "wasm:naked.wasm";
                connection_string.serialize(ref inputs);
                let selector: ByteArray = "naked:adder/add@0.1.0/add";
                selector.serialize(ref inputs);
                (1_i64).serialize(ref inputs);
                (2_i64).serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<i64>(result);

                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "wasm:naked.wasm";
                connection_string.serialize(ref inputs);
                let selector: ByteArray = "adda";
                selector.serialize(ref inputs);
                (1_i64).serialize(ref inputs);
                (2_i64).serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<i64>(result);

                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "wasm:naked.wasm";
                connection_string.serialize(ref inputs);
                let selector: ByteArray = "f";
                selector.serialize(ref inputs);
                (1_i32).serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<i32>(result);

                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "wasm:naked.wasm";
                connection_string.serialize(ref inputs);
                let selector: ByteArray = "naked:adder/add@0.1.0/f";
                selector.serialize(ref inputs);
                (1_i32).serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<i32>(result);

                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "wasm:naked.wasm";
                connection_string.serialize(ref inputs);
                let selector: ByteArray = "naked:adder/ambiguous@0.1.0/f";
                selector.serialize(ref inputs);
                (1_i32).serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<i32>(result);
            }
        "#})
        .asset("naked.wasm", include_bytes!("naked.wasm"))
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Result::Ok(3)
            Result::Ok(3)
            Result::Err("no exported func in component named: adda
            note: available funcs are: naked:adder/add@0.1.0/add, naked:adder/add@0.1.0/f, naked:adder/ambiguous@0.1.0/f")
            Result::Err("multiple exports named: f
            note: possible matches are: naked:adder/add@0.1.0/f, naked:adder/ambiguous@0.1.0/f")
            Result::Ok(2)
            Result::Ok(1001)
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}

#[test]
fn trap() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "wasm:trap.wasm";
                connection_string.serialize(ref inputs);
                let selector: ByteArray = "trap";
                selector.serialize(ref inputs);
                array![true, false].serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<u64>(result);
            }
        "#})
        .asset("trap.wasm", include_bytes!("trap.wasm"))
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Result::Err("error while executing at wasm backtrace:
                [..]

            Caused by:
                wasm trap: wasm `unreachable` instruction executed")
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}

#[test]
fn out_of_tree_asset() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "wasm:foo/../../exploit.wasm";
                connection_string.serialize(ref inputs);
                let selector: ByteArray = "exploit";
                selector.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<()>(result);
            }
        "#})
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Result::Err("invalid asset path `foo/../../exploit.wasm`: parent reference `..` points outside of base directory")
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}
