use assert_fs::TempDir;
use indoc::indoc;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

// TODO(mkaput): Remove starknet dependency in favour of the oracle package.

#[test]
fn oracle_invoke_without_experimental_flag_fails() {
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("oracle_test")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [executable]
            
            [cairo]
            enable-gas = false
        "#})
        .dep_cairo_execute()
        .dep_starknet()
        .lib_cairo(indoc! {r#"
        #[executable]
        fn main() {
            starknet::testing::cheatcode::<'oracle_invoke'>(array![].span());
        }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("execute")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            error: Cairo program run failed: Error at pc=0:33:
            Got an exception while executing a hint: Oracles are experimental feature. To enable, pass --experimental-oracles CLI flag.
            Cairo traceback (most recent call last):
            Unknown location (pc=0:2)
            Unknown location (pc=0:10)

        "#});
}

#[test]
fn oracle_invoke_direct() {
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("oracle_test")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [executable]
            
            [cairo]
            enable-gas = false
        "#})
        .dep_cairo_execute()
        .dep_starknet()
        .lib_cairo(indoc! {r#"
        #[executable]
        fn main() {
            let mut inputs: Array<felt252> = array![];
            let connection_string: ByteArray = "connection string";
            connection_string.serialize(ref inputs);
            'pow'.serialize(ref inputs);
            (4).serialize(ref inputs);
            (2).serialize(ref inputs);
            starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span());
        }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("execute")
        .arg("--experimental-oracles")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Compiling oracle_test v0.1.0 ([..]/Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing oracle_test
            Saving output to: target/execute/oracle_test/execution1
        "#});
}
