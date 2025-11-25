use assert_fs::TempDir;
use indoc::formatdoc;
use scarb_test_support::command::Scarb;
// use scarb_test_support::filesystem::dump_temp;
use scarb_test_support::project_builder::ProjectBuilder;
mod markdown_target;
use markdown_target::MarkdownTargetChecker;

const CODE_WITH_SNIPPETS: &str = include_str!("code/code_12.cairo");
const EXPECTED_CODE_WITH_SNIPPETS_PATH: &str = "tests/data/runnable_snippets";

#[test]
fn supports_runnable_examples() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(CODE_WITH_SNIPPETS)
        .build(&t);

    Scarb::quick_command()
        .arg("doc")
        .args(["--output-format", "markdown"])
        .arg("--build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(formatdoc! {r#"
            [..] Executing snippet #0 from `hello_world::foo_bar`
            [..] Compiling hello_world_snippet_0 v0.1.0 ([..])
            [..]  Finished `dev` profile target(s) in [..]
            [..] Executing hello_world_snippet_0
            foo
            bar
            Saving output to: target/execute/hello_world_snippet_0/execution1
            Saving output to: target/doc/hello_world
            Saving build output to: target/doc/hello_world/book

            Run the following to see the results:[..]
            `mdbook serve target/doc/hello_world`

            Or open the following in your browser:[..]
            `[..]/target/doc/hello_world/book/index.html`
        "#});

    MarkdownTargetChecker::lenient()
        .actual(t.path().join("target/doc/hello_world").to_str().unwrap())
        .expected(EXPECTED_CODE_WITH_SNIPPETS_PATH)
        .assert_all_files_match();
}
