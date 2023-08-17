use assert_fs::TempDir;
use indoc::indoc;

use scarb_test_support::command::Scarb;
use scarb_test_support::filesystem::{path_with_temp_dir, write_simple_hello_script};
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
#[cfg_attr(
    not(target_family = "unix"),
    ignore = "This test should write a Rust code, because currently it only assumes Unix."
)]
fn delegates_to_cairo_test() {
    let t = TempDir::new().unwrap();
    write_simple_hello_script("cairo-test", &t);

    ProjectBuilder::start().build(&t);

    Scarb::quick_snapbox()
        .args(["test", "beautiful", "world"])
        .env("PATH", path_with_temp_dir(&t))
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        Running tests for package: pkg0
        Hello beautiful world
        "#});
}

#[test]
fn prefers_test_script() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .manifest_extra(indoc! {r#"
        [scripts]
        test = "echo 'Hello from script'"
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .args(["test", "beautiful", "world"])
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        Running tests for package: pkg0
        Hello from script beautiful world
        "#});
}

#[test]
#[cfg_attr(
    target_family = "windows",
    ignore = "Something is fishy with PATH variable on our Windows CI, because it contains target/debug directory, even duplicated."
)]
fn errors_when_missing_script_and_cairo_test() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().build(&t);

    Scarb::new()
        .isolate_from_extensions()
        .snapbox()
        .args(["test", "beautiful", "world"])
        .env("PATH", path_with_temp_dir(&t))
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
        Running tests for package: pkg0
        error: no such command: `cairo-test`
        "#});
}
