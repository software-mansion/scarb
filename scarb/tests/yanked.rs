use assert_fs::TempDir;
use assert_fs::prelude::*;
use indoc::indoc;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
use scarb_test_support::registry::local::{LocalRegistry, yank};

#[test]
fn will_use_yanked_if_already_present_in_lockfile() {
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
        .lib_cairo(indoc! {r#"fn hello() -> felt252 { 0 }"#})
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

    yank(registry.t.child("index/3/f/foo.json").path(), "1.0.0").unwrap();

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
fn will_not_use_yanked_version() {
    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("foo")
            .version("1.0.0")
            .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
            .build(t);
    });
    yank(registry.t.child("index/3/f/foo.json").path(), "1.0.0").unwrap();

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .version("1.0.0")
        .dep("foo", Dep.version("1.0.0").registry(&registry))
        .lib_cairo(indoc! {r#"fn hello() -> felt252 { 0 }"#})
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

    registry.publish(|t| {
        ProjectBuilder::start()
            .name("foo")
            .version("1.0.2")
            .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
            .build(t);
    });

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    let lockfile = t.child("Scarb.lock");
    lockfile.assert(predicates::str::contains(indoc! {r#"
        [[package]]
        name = "foo"
        version = "1.0.2"
    "#}));
}
