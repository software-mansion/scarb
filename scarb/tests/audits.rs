use assert_fs::TempDir;
use assert_fs::prelude::*;
use indoc::indoc;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
use scarb_test_support::registry::local::{LocalRegistry, audit, unaudit};
use scarb_test_support::workspace_builder::WorkspaceBuilder;

#[test]
fn require_audits_disallows_non_audited_version() {
    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("foo")
            .version("1.0.0")
            .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
            .build(t);
    });
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello_world")
        .version("1.0.0")
        .dep("foo", Dep.version("1.0.0").registry(&registry))
        .lib_cairo(r#"fn hello() -> felt252 { 0 }"#)
        .manifest_extra(
            r#"
            [security]
            require-audits = true
        "#,
        )
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: cannot get dependencies of `hello_world@1.0.0`

        Caused by:
            cannot find package `foo ^1.0.0`
        "#});

    audit(registry.t.child("index/3/f/foo.json").path(), "1.0.0").unwrap();
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();
    let lockfile = t.child("Scarb.lock");
    lockfile.assert(predicates::str::contains(indoc! {r#"
        [[package]]
        name = "foo"
        version = "1.0.0"
    "#}));
}

#[test]
fn require_audits_disallows_non_audited_version_transitive() {
    let mut registry = LocalRegistry::create();

    registry.publish(|t| {
        ProjectBuilder::start()
            .name("foo")
            .version("1.0.0")
            .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
            .build(t);
    });

    let registry_url = registry.to_string();

    registry.publish(|t| {
        ProjectBuilder::start()
            .name("bar")
            .version("1.0.0")
            .dep("foo", Dep.version("1.0.0").registry(&registry_url))
            .lib_cairo(r#"fn g() -> felt252 { 0 }"#)
            .build(t);
    });

    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello_world")
        .version("1.0.0")
        .dep("bar", Dep.version("1.0.0").registry(&registry_url))
        .lib_cairo(r#"fn hello() -> felt252 { 0 }"#)
        .manifest_extra(
            r#"
            [security]
            require-audits = true
        "#,
        )
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: cannot get dependencies of `hello_world@1.0.0`

            Caused by:
                cannot find package `bar ^1.0.0`
        "#});

    audit(registry.t.child("index/3/b/bar.json").path(), "1.0.0").unwrap();
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: cannot get dependencies of `bar@1.0.0`

            Caused by:
                cannot find package `foo ^1.0.0`
        "#});
}

#[test]
fn require_audits_allows_audited_version() {
    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("foo")
            .version("1.0.0")
            .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
            .build(t);
    });
    audit(registry.t.child("index/3/f/foo.json").path(), "1.0.0").unwrap();
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello_world")
        .version("1.0.0")
        .dep("foo", Dep.version("1.0.0").registry(&registry))
        .lib_cairo(r#"fn hello() -> felt252 { 0 }"#)
        .manifest_extra(
            r#"
            [security]
            require-audits = true
        "#,
        )
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    let lockfile = t.child("Scarb.lock");
    lockfile.assert(predicates::str::contains(indoc! {r#"
        [[package]]
        name = "foo"
        version = "1.0.0"
    "#}));
    unaudit(registry.t.child("index/3/f/foo.json").path(), "1.0.0").unwrap();
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: cannot get dependencies of `hello_world@1.0.0`

        Caused by:
            cannot find package `foo ^1.0.0`
    "#});
}

#[test]
fn require_audits_workspace() {
    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("foo")
            .version("1.0.0")
            .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
            .build(t);
    });
    let t = TempDir::new().unwrap();
    let first = t.child("first");
    let second = t.child("second");

    ProjectBuilder::start()
        .name("first")
        .version("1.0.0")
        .dep("foo", Dep.version("1.0.0").registry(&registry))
        .lib_cairo(r#"fn hello() -> felt252 { 0 }"#)
        .manifest_extra(
            r#"
            [security]
            require-audits.workspace = true
        "#,
        )
        .build(&first);

    ProjectBuilder::start()
        .name("second")
        .version("1.0.0")
        .lib_cairo(r#"fn hello() -> felt252 { 0 }"#)
        .build(&second);

    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .manifest_extra(
            r#"
            [workspace.security]
            require-audits = true
        "#,
        )
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .arg("--workspace")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: cannot get dependencies of `first@1.0.0`

            Caused by:
                cannot find package `foo ^1.0.0`
        "#});

    audit(registry.t.child("index/3/f/foo.json").path(), "1.0.0").unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .arg("--workspace")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&t)
        .assert()
        .success();
}
