use assert_fs::prelude::*;
use assert_fs::TempDir;
use git_repository::refs::transaction::PreviousValue;
use indoc::{formatdoc, indoc};

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

#[test]
fn compile_git_dep_branch() {
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep1")
            .lib_cairo("fn hello() -> felt { 42 }")
            .build(&t)
    });

    git_dep.git(["checkout", "-b", "foo"]);
    git_dep
        .child("src/lib.cairo")
        .write_str("fn branched() -> felt { 53 }")
        .unwrap();
    git_dep.commit();

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep(
            "dep1",
            formatdoc! {r#"
                git = "{git_dep}"
                branch = "foo"
            "#},
        )
        .lib_cairo("fn world() -> felt { dep1::branched() }")
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
}

#[test]
fn compile_git_dep_tag() {
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep1")
            .lib_cairo("fn hello() -> felt { 42 }")
            .build(&t)
    });

    git_dep
        .child("src/lib.cairo")
        .write_str("fn tagged() -> felt { 53 }")
        .unwrap();
    git_dep.commit();
    git_dep.git(["tag", "-a", "v1.4.0", "-m", "first tag"]);

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep(
            "dep1",
            formatdoc! {r#"
                git = "{git_dep}"
                tag = "v1.4.0"
            "#},
        )
        .lib_cairo("fn world() -> felt { dep1::tagged() }")
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
}

#[test]
fn compile_git_dep_pull_request() {
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep1")
            .lib_cairo("fn hello() -> felt { 42 }")
            .build(&t)
    });

    let repo = git_repository::open(git_dep.p.path()).unwrap();
    repo.reference(
        "refs/pull/330/head",
        repo.head_id().unwrap(),
        PreviousValue::Any,
        "open pull request",
    )
    .unwrap();

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep(
            "dep1",
            formatdoc! {r#"
                git = "{git_dep}"
                rev = "refs/pull/330/head"
            "#},
        )
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
}
