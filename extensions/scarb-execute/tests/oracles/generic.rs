use crate::support::CheckBuilder;
use indoc::indoc;

#[test]
fn no_experimental_flag() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {
                starknet::testing::cheatcode::<'oracle_invoke'>(array![].span());
            }
        "#})
        .enable_experimental_oracles_flag(false)
        .profile("release".to_string())
        .failure()
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `release` profile target(s) in [..]
            [..]Executing oracle_test
            error: Cairo program run failed: Error at pc=0:33:
            Got an exception while executing a hint: Oracles are experimental feature. To enable, pass --experimental-oracles CLI flag.
            Cairo traceback (most recent call last):
            Unknown location (pc=0:2)
            Unknown location (pc=0:10)

        "#})
        .check();
}

#[test]
fn unknown_scheme() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "unknown:///test";
                connection_string.serialize(ref inputs);
                'foo'.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<ByteArray>(result);
            }
        "#})
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Result::Err("unsupported connection scheme: "unknown:///test"
            note: supported schemes are: [..]")
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}

#[test]
fn no_scheme() {
    CheckBuilder::default()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() {
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "no scheme";
                connection_string.serialize(ref inputs);
                'foo'.serialize(ref inputs);
                let result = starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
                oracle_asserts::print::<ByteArray>(result);
            }
        "#})
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Result::Err("unsupported connection scheme: "no scheme"
            note: supported schemes are: [..]")
            Saving output to: target/execute/oracle_test/execution1
        "#})
        .check();
}
