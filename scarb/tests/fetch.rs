use indoc::indoc;
use scarb_test_support::gitx;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn simple() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn check_git_fetch_stdout() {
    let t = assert_fs::TempDir::new().unwrap();

    let git_dep = gitx::new("dep1", |t| ProjectBuilder::start().name("dep1").build(&t));

    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .dep("dep1", &git_dep)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep1
        "#});
}
