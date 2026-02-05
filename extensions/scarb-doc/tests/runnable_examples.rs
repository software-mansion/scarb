use assert_fs::TempDir;
use indoc::{formatdoc, indoc};
use scarb_test_support::command::Scarb;
// use scarb_test_support::filesystem::dump_temp;
use scarb_test_support::project_builder::ProjectBuilder;
mod markdown_target;
use markdown_target::MarkdownTargetChecker;

const CODE_WITH_RUNNABLE_CODE_BLOCKS: &str = include_str!("code/code_12.cairo");
const CODE_WITH_COMPILE_ERROR: &str = include_str!("code/code_13.cairo");
const CODE_WITH_RUNTIME_ERROR: &str = include_str!("code/code_14.cairo");
const CODE_WITH_MULTIPLE_CODE_BLOCKS_PER_ITEM: &str = include_str!("code/code_15.cairo");
const CODE_WITH_STARKNET_CONTRACT: &str = include_str!("code/code_16.cairo");
const EXPECTED_WITH_EMBEDDINGS_PATH: &str = "tests/data/runnable_examples";
const EXPECTED_MULTIPLE_PER_ITEM_PATH: &str = "tests/data/runnable_examples_multiple_per_item";

#[test]
fn supports_runnable_examples() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(CODE_WITH_RUNNABLE_CODE_BLOCKS)
        .build(&t);

    Scarb::quick_command()
        .arg("doc")
        .args(["--output-format", "markdown"])
        .arg("--disable-remote-linking")
        .arg("--build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(formatdoc! {r#"
            [..] Running 3 doc examples for `hello_world`
            test hello_world::bar ... ignored
            test hello_world::foo ... ignored
            [..] Compiling hello_world_example_1 v0.1.0 ([..])
            [..]  Finished `dev` profile target(s) in [..]
            test hello_world::foo_bar ... ok

            test result: ok. 1 passed; 0 failed; 2 ignored
            Saving output to: target/doc/hello_world
            Saving build output to: target/doc/hello_world/book

            Run the following to see the results:[..]
            `mdbook serve target/doc/hello_world`

            Or open the following in your browser:[..]
            `[..]/target/doc/hello_world/book/index.html`
        "#});

    MarkdownTargetChecker::lenient()
        .actual(t.path().join("target/doc/hello_world").to_str().unwrap())
        .expected(EXPECTED_WITH_EMBEDDINGS_PATH)
        .assert_all_files_match();
}

#[test]
fn supports_runnable_examples_multiple_per_item() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(CODE_WITH_MULTIPLE_CODE_BLOCKS_PER_ITEM)
        .build(&t);

    Scarb::quick_command()
        .arg("doc")
        .args(["--output-format", "markdown"])
        .arg("--disable-remote-linking")
        .arg("--build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(formatdoc! {r#"
            [..] Running 2 doc examples for `hello_world`
            [..] Compiling hello_world_example_1 v0.1.0 ([..])
            [..]  Finished `dev` profile target(s) in [..]
            test hello_world::add (example 0) ... ok
            [..] Compiling hello_world_example_2 v0.1.0 ([..])
            [..]  Finished `dev` profile target(s) in [..]
            test hello_world::add (example 1) ... ok

            test result: ok. 2 passed; 0 failed; 0 ignored
            Saving output to: target/doc/hello_world
            Saving build output to: target/doc/hello_world/book

            Run the following to see the results:[..]
            `mdbook serve target/doc/hello_world`

            Or open the following in your browser:[..]
            `[..]/target/doc/hello_world/book/index.html`
        "#});

    MarkdownTargetChecker::lenient()
        .actual(t.path().join("target/doc/hello_world").to_str().unwrap())
        .expected(EXPECTED_MULTIPLE_PER_ITEM_PATH)
        .assert_all_files_match();
}

#[test]
fn runnable_example_fails_at_compile_time() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(CODE_WITH_COMPILE_ERROR)
        .build(&t);

    Scarb::quick_command()
        .arg("doc")
        .args(["--output-format", "markdown"])
        .arg("--disable-remote-linking")
        .arg("--build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..] Running 1 doc examples for `hello_world`
            [..] Compiling hello_world_example_1 v0.1.0 ([..])
            error[E0006]: Function not found.
             --> [..]lib.cairo[..]
                undefined();
                ^^^^^^^^^

            error: could not compile `hello_world_example_1` due to 1 previous error
            test hello_world::foo ... FAILED

            failures:
                hello_world::foo

            test result: FAILED. 0 passed; 1 failed; 0 ignored
            error: doc tests failed
        "#});
}

#[test]
fn runnable_example_fails_at_runtime() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(CODE_WITH_RUNTIME_ERROR)
        .build(&t);

    Scarb::quick_command()
        .arg("doc")
        .args(["--output-format", "markdown"])
        .arg("--disable-remote-linking")
        .arg("--build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..] Running 1 doc examples for `hello_world`
            [..] Compiling hello_world_example_1 v0.1.0 ([..])
            [..]  Finished `dev` profile target(s) in [..]
            test hello_world::foo ... FAILED

            failures:
                hello_world::foo

            test result: FAILED. 0 passed; 1 failed; 0 ignored
            error: doc tests failed
        "#});
}

#[test]
fn supports_runnable_examples_with_starknet_contract() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .edition("2023_01")
        .manifest_extra(indoc! {r#"
            [lib]
            [[target.starknet-contract]]
        "#})
        .dep_starknet()
        .lib_cairo(CODE_WITH_STARKNET_CONTRACT)
        .build(&t);

    Scarb::quick_command()
        .arg("doc")
        .args(["--output-format", "markdown"])
        .arg("--disable-remote-linking")
        .arg("--build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(formatdoc! {r#"
            [..] Running 1 doc examples for `hello_world`
            [..] Compiling hello_world_example_1 v0.1.0 ([..])
            [..]  Finished `dev` profile target(s) in [..]
            test hello_world::IBalance ... ok

            test result: ok. 1 passed; 0 failed; 0 ignored
            Saving output to: target/doc/hello_world
            Saving build output to: target/doc/hello_world/book

            Run the following to see the results:[..]
            `mdbook serve target/doc/hello_world`

            Or open the following in your browser:[..]
            `[..]/target/doc/hello_world/book/index.html`
        "#});
}
