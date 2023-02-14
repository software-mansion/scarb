use assert_fs::prelude::*;
use assert_fs::TempDir;
use indoc::indoc;

use crate::support::command::Scarb;
use crate::support::fsx::ChildPathEx;
use crate::support::gitx;
use crate::support::project_builder::ProjectBuilder;

#[test]
fn compile_simple_git_dep() {
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep1")
            .lib_cairo("fn hello() -> felt { 42 }")
            .build(&t)
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep1", &git_dep)
        .lib_cairo("fn world() -> felt { dep1::hello() }")
        .build(&t);

    let config = Scarb::test_config(t.child("Scarb.toml"));

    Scarb::from_config(&config)
        .snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep1
        [..] Compiling hello v1.0.0 ([..])
        [..]  Finished release target(s) in [..]
        "#});

    assert_eq!(t.child("target/release").files(), vec!["hello.sierra"]);
}
