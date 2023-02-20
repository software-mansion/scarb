use std::fs;

use assert_fs::prelude::*;
use assert_fs::TempDir;
use gix::refs::transaction::PreviousValue;
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

    Scarb::quick_snapbox()
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

    git_dep.checkout_branch("foo");
    git_dep.change_file("src/lib.cairo", "fn branched() -> felt { 53 }");

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

    Scarb::quick_snapbox()
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

    git_dep.change_file("src/lib.cairo", "fn tagged() -> felt { 53 }");
    git_dep.tag("v1.4.0");

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

    Scarb::quick_snapbox()
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

    let repo = gix::open(git_dep.p.path()).unwrap();
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

    Scarb::quick_snapbox()
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
fn compile_with_nested_paths() {
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep1")
            .lib_cairo("fn hello() -> felt { dep2::hello() }")
            .dep("dep2", r#" path = "vendor/dep2" "#)
            .build(&t);

        ProjectBuilder::start()
            .name("dep2")
            .lib_cairo("fn hello() -> felt { 42 }")
            .build(&t.child("vendor/dep2"));
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep1", &git_dep)
        .lib_cairo("fn world() -> felt { dep1::hello() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();
}

// TODO(mkaput): Redo TomlDependency deserializer to stick parsing particular variant
//   if specific keyword appears.
#[test]
fn compile_with_short_ssh_git() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep", r#" git = "git@github.com:a/dep" "#)
        .lib_cairo("fn world() -> felt { dep1::hello() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: failed to parse manifest at `[..]`

            Caused by:
                TOML parse error at line 2, column 24
                  |
                2 | dependencies = { dep = { git = "git@github.com:a/dep" } }
                  |                        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
                data did not match any variant of untagged enum TomlDependency
        "#});
}

// TODO(mkaput): Add tests with submodules.
// TODO(mkaput): Add tests with `scarb update`.

#[test]
fn stale_cached_version() {
    let dep = gitx::new("dep", |t| {
        ProjectBuilder::start()
            .name("dep")
            .lib_cairo("fn hello() -> felt { 11111111111101 }")
            .build(&t)
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep", &dep)
        .lib_cairo("fn world() -> felt { dep::hello() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep
        [..] Compiling hello v1.0.0 ([..])
        [..]  Finished release target(s) in [..]
        "#});

    t.child("target/release/hello.sierra")
        .assert(predicates::str::contains("11111111111101"));

    // TODO(mkaput): Lockfile should prevent updating.
    //   When lockfile will be implemented, uncomment this and implement missing parts.
    // Scarb::quick_snapbox()
    //     .arg("build")
    //     .current_dir(&t)
    //     .assert()
    //     .success()
    //     .stdout_matches(indoc! {r#"
    //     [..] Compiling hello v1.0.0 ([..])
    //     [..]  Finished release target(s) in [..]
    //     "#});
    //
    // t.child("target/release/hello.sierra")
    //     .assert(predicates::str::contains("11111111111101"));
    //
    // remove lockfile here

    dep.change_file("src/lib.cairo", "fn hello() -> felt { 11111111111102 }");

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep
        [..] Compiling hello v1.0.0 ([..])
        [..]  Finished release target(s) in [..]
        "#});

    t.child("target/release/hello.sierra")
        .assert(predicates::str::contains("11111111111102"));
}

#[test]
fn change_source() {
    let dep = gitx::new("dep", |t| {
        ProjectBuilder::start()
            .name("dep")
            .lib_cairo("fn y() -> felt { 1 }")
            .build(&t);
    });

    dep.tag("v1.0.0");

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.0.1")
        .dep("dep", &dep)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep
        [..] Compiling hello v0.0.1 ([..])
        [..]  Finished release target(s) in [..]
        "#});

    dep.change_file("src/lib.cairo", "fn x() -> felt { 0 }");
    dep.tag("v2.0.0");

    let manifest = t.child("Scarb.toml");
    let manifest_toml = fs::read_to_string(manifest.path()).unwrap();
    let manifest_toml = manifest_toml.replace("1.0.0", "2.0.0");
    manifest.write_str(&manifest_toml).unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep
        [..] Compiling hello v0.0.1 ([..])
        [..]  Finished release target(s) in [..]
        "#});
}

#[test]
fn force_push() {
    let dep = gitx::new("dep", |t| {
        ProjectBuilder::start()
            .name("dep")
            .lib_cairo("fn hello() -> felt { 42 }")
            .build(&t)
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep", &dep)
        .lib_cairo("fn world() -> felt { dep::hello() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    dep.child("src/lib.cairo")
        .write_str("fn hello() -> felt { 43 }")
        .unwrap();

    dep.git(["add", "."]);
    dep.git(["commit", "--amend", "-m", "amended"]);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep
        [..] Compiling hello v1.0.0 ([..])
        [..]  Finished release target(s) in [..]
        "#});
}
