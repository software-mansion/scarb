use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
use indoc::indoc;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;
use scarb_test_support::workspace_builder::WorkspaceBuilder;

#[test]
fn list_simple() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("1.0.0")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--list")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            VERSION
            Scarb.orig.toml
            Scarb.toml
            src/lib.cairo
        "#});
}

#[test]
fn list_workspace() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
        .build(&t.child("first"));
    ProjectBuilder::start()
        .name("second")
        .build(&t.child("second"));
    WorkspaceBuilder::start()
        // Trick to test if packages are sorted alphabetically by name in the output.
        .add_member("second")
        .add_member("first")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--list")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            first:
            VERSION
            Scarb.orig.toml
            Scarb.toml
            src/lib.cairo

            second:
            VERSION
            Scarb.orig.toml
            Scarb.toml
            src/lib.cairo
        "#});
}
