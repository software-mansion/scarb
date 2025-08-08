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
        // Cairo-test dumps the failure message to stderr.
        // It is mixed with Scarb logs, so we're not going to match on it.
        .stdout_matches(indoc! {r#"
            [..] Compiling test(oracle_test_unittest) oracle_test v0.1.0 ([..])
            [..] Finished `dev` profile target(s) in [..] seconds
            [..] Testing oracle_test
            running 1 test
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
        .cp_test_oracle("test_oracle.py")
        .lib_cairo(indoc! {r#"
            #[test]
            fn it_works() {
                let connection_string: ByteArray = "stdio:python3 ./test_oracle.py";

                let mut inputs: Array<felt252> = array![];
                connection_string.serialize(ref inputs);
                'sqrt'.serialize(ref inputs);
                (16).serialize(ref inputs);
                let result = oracle_asserts::deserialize::<felt252>(
                    starknet::testing::cheatcode::<'oracle_invoke'>(inputs.span())
                );
                assert_eq!(result, Ok(4));
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
