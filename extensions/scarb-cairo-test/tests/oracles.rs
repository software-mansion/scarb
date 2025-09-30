use assert_fs::TempDir;
use indoc::indoc;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn oracle_invoke_without_experimental_flag_fails() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("oracle_test")
        .version("0.1.0")
        .dep_cairo_test()
        .lib_cairo(indoc! {r#"
            #[test]
            fn should_not_work() {
                starknet::testing::cheatcode::<'oracle_invoke'>(array![].span());
            }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("cairo-test")
        .env("RUST_BACKTRACE", "0")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            [..] Compiling test(oracle_test_unittest) oracle_test v0.1.0 ([..])
            [..] Finished `dev` profile target(s) in [..] seconds
            [..] Testing oracle_test
            running 1 test
            test oracle_test::should_not_work ... failed to run
            failures:
               oracle_test::should_not_work - Got an exception while executing a hint: Oracles are experimental feature. To enable, pass --experimental-oracles CLI flag.

        "#});
}

#[test]
fn oracle() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("oracle_test")
        .version("0.1.0")
        .dep_cairo_test()
        .dep_builtin("assert_macros")
        .dep_starknet()
        .dep_oracle_asserts()
        .lib_cairo(indoc! {r#"
            #[test]
            fn it_works() {
                let mut inputs: Array<felt252> = array![];
                let connection_string: ByteArray = "shell:";
                connection_string.serialize(ref inputs);
                'exec'.serialize(ref inputs);
                let command: ByteArray = "echo hello";
                command.serialize(ref inputs);
                let result = oracle_asserts::deserialize::<(i32, ByteArray)>(
                    starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span())
                );
                assert_eq!(result, Ok((0, "hello\n")));
            }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("cairo-test")
        .arg("--experimental-oracles")
        .env("RUST_BACKTRACE", "0")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..] Compiling test(oracle_test_unittest) oracle_test v0.1.0 ([..])
            [..] Finished `dev` profile target(s) in [..] seconds
            [..] Testing oracle_test
            running 1 test
            test oracle_test::it_works ... ok (gas usage est.: [..])
            test result: ok. 1 passed; 0 failed; 0 ignored; 0 filtered out;
        "#});
}
