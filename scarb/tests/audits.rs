use assert_fs::TempDir;
use assert_fs::prelude::*;
use indoc::{formatdoc, indoc};
use scarb_test_support::command::Scarb;
use scarb_test_support::gitx;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
use scarb_test_support::registry::local::{LocalRegistry, audit, unaudit};
use scarb_test_support::workspace_builder::WorkspaceBuilder;

#[test]
fn require_audits_allows_non_audited_dev_dep() {
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
        .dev_dep("foo", Dep.version("1.0.0").registry(&registry))
        .lib_cairo(r#"fn hello() -> felt252 { 0 }"#)
        .manifest_extra(
            r#"
            [workspace]
            require-audits = true
        "#,
        )
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
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
fn require_audits_allows_non_audited_dev_dep_with_patch() {
    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("foo")
            .version("1.0.0")
            .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
            .build(t);
    });
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("foo")
            .version("2.0.0")
            .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
            .build(t);
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .version("1.0.0")
        .dev_dep("foo", Dep.version("1").registry(&registry))
        .lib_cairo(indoc! {r#"fn hello() -> felt252 { 0 }"#})
        .manifest_extra(formatdoc! {r#"
            [workspace]
            require-audits = true

            [patch."{}"]
            foo = {}
        "#, registry.url.clone(), Dep.version("2").registry(&registry).build()})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success();

    let lockfile = t.child("Scarb.lock");
    lockfile.assert(predicates::str::contains(indoc! {r#"
        [[package]]
        name = "foo"
        version = "2.0.0"
    "#}));
}

#[test]
fn require_audits_allows_audited_version_only() {
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
            [workspace]
            require-audits = true
        "#,
        )
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: version solving failed:
        Because there is no version of foo in >=1.0.0, <2.0.0 and hello_world 1.0.0 depends on foo >=1.0.0, <2.0.0, hello_world 1.0.0 is forbidden.
        "#});

    audit(registry.t.child("index/3/f/foo.json").path(), "1.0.0").unwrap();

    Scarb::quick_snapbox()
        .arg("fetch")
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
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: version solving failed:
        Because there is no version of foo in >=1.0.0, <2.0.0 and hello_world 1.0.0 depends on foo >=1.0.0, <2.0.0, hello_world 1.0.0 is forbidden.
    "#});
}

#[test]
fn require_audits_disallows_git_dep() {
    let git_dep = gitx::new("foo", |t| {
        ProjectBuilder::start()
            .name("foo")
            .lib_cairo("pub fn hello() -> felt252 { 42 }")
            .build(&t)
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("foo", &git_dep)
        .lib_cairo("fn world() -> felt252 { foo::hello() }")
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
            error: dependency `foo` from `git` source is not allowed when audit requirement is enabled

            Caused by:
                0: dependency `foo` from `git` source is not allowed when audit requirement is enabled
                1: help: depend on a registry package
                   alternatively, consider whitelisting dependency in package manifest
                    --> Scarb.toml
                       [security]
                       allow-no-audits = ["foo"]
        "#});
}

#[test]
fn require_audits_disallows_path_dep() {
    let t = TempDir::new().unwrap();

    let foo = t.child("foo");
    ProjectBuilder::start()
        .name("foo")
        .version("0.1.0")
        .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
        .build(&foo);

    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .dep("foo", Dep.path("foo"))
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
            error: dependency `foo` from `path` source is not allowed when audit requirement is enabled

            Caused by:
                0: dependency `foo` from `path` source is not allowed when audit requirement is enabled
                1: help: depend on a registry package
                   alternatively, consider whitelisting dependency in package manifest
                    --> Scarb.toml
                       [security]
                       allow-no-audits = ["foo"]
        "#});
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
            [workspace]
            require-audits = true
        "#,
        )
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: version solving failed:
            Because there is no version of bar in >=1.0.0, <2.0.0 and hello_world 1.0.0 depends on bar >=1.0.0, <2.0.0, hello_world 1.0.0 is forbidden.
        "#});

    audit(registry.t.child("index/3/b/bar.json").path(), "1.0.0").unwrap();
    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: version solving failed:
            Because there is no version of foo in >=1.0.0, <2.0.0 and bar 1.0.0 depends on foo >=1.0.0, <2.0.0, bar 1.0.0 is forbidden.
            And because there is no version of bar in >1.0.0, <2.0.0 and hello_world 1.0.0 depends on bar >=1.0.0, <2.0.0, hello_world 1.0.0 is forbidden.
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
    let hello = t.child("hello");

    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("foo", Dep.version("1.0.0").registry(&registry))
        // The workspace-level `require-audits` should override this one.
        .manifest_extra(
            r#"
            [workspace]
            require-audits = false
        "#,
        )
        .lib_cairo(r#"fn hello() -> felt252 { 0 }"#)
        .build(&hello);

    WorkspaceBuilder::start()
        .add_member("hello")
        .require_audits(true)
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
            error: version solving failed:
            Because there is no version of foo in >=1.0.0, <2.0.0 and hello 1.0.0 depends on foo >=1.0.0, <2.0.0, hello 1.0.0 is forbidden.
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

#[test]
fn require_audits_workspace_normal_and_dev_dep() {
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
    ProjectBuilder::start()
        .name("first")
        .dev_dep("foo", Dep.version("1.0.0").registry(&registry))
        .build(&first);

    let second = t.child("second");
    ProjectBuilder::start()
        .name("second")
        .dep("foo", Dep.version("1.0.0").registry(&registry))
        .build(&second);

    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .require_audits(true)
        .build(&t);

    // Having a dev dep in a workspace should not lift the audit requirement for a normal dep.
    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure();
}

#[test]
fn will_update_to_audited_version_only() {
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
            [workspace]
            require-audits = true
        "#,
        )
        .build(&t);

    audit(registry.t.child("index/3/f/foo.json").path(), "1.0.0").unwrap();

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success();

    let lockfile = t.child("Scarb.lock");
    lockfile.assert(predicates::str::contains(indoc! {r#"
        [[package]]
        name = "foo"
        version = "1.0.0"
    "#}));

    registry.publish(|t| {
        ProjectBuilder::start()
            .name("foo")
            .version("1.1.0")
            .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
            .build(t);
    });

    // Locked version should not change since the new version is not audited.
    Scarb::quick_snapbox()
        .arg("update")
        .current_dir(&t)
        .assert()
        .success();

    lockfile.assert(predicates::str::contains(indoc! {r#"
        [[package]]
        name = "foo"
        version = "1.0.0"
    "#}));

    audit(registry.t.child("index/3/f/foo.json").path(), "1.1.0").unwrap();

    // Update should now pick the audited version.
    Scarb::quick_snapbox()
        .arg("update")
        .current_dir(&t)
        .assert()
        .success();

    lockfile.assert(predicates::str::contains(indoc! {r#"
        [[package]]
        name = "foo"
        version = "1.1.0"
    "#}));
}

#[test]
fn bypass_audit_requirement() {
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

    // Whitelist direct dependency only
    let first = t.child("first");
    ProjectBuilder::start()
        .name("first")
        .version("1.0.0")
        .dep("bar", Dep.version("1.0.0").registry(&registry))
        .lib_cairo(indoc! {r#"fn hello() -> felt252 { 0 }"#})
        .manifest_extra(
            r#"
            [workspace]
            require-audits = true
            allow-no-audits = ["bar"]
        "#,
        )
        .build(&first);

    // Bypassing audit requirement is not transitive
    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&first)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: version solving failed:
            Because there is no version of foo in >=1.0.0, <2.0.0 and bar 1.0.0 depends on foo >=1.0.0, <2.0.0, bar 1.0.0 is forbidden.
            And because there is no version of bar in >1.0.0, <2.0.0 and first 1.0.0 depends on bar >=1.0.0, <2.0.0, first 1.0.0 is forbidden.
        "#});

    // Now whitelist both direct and transitive dependencies
    let second = t.child("second");
    ProjectBuilder::start()
        .name("second")
        .version("1.0.0")
        .dep("bar", Dep.version("1.0.0").registry(&registry))
        .lib_cairo(indoc! {r#"fn hello() -> felt252 { 0 }"#})
        .manifest_extra(
            r#"
            [workspace]
            require-audits = true
            allow-no-audits = ["bar", "foo"]
        "#,
        )
        .build(&second);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&second)
        .assert()
        .success();

    let lockfile = second.child("Scarb.lock");
    lockfile.assert(predicates::str::contains(indoc! {r#"
        [[package]]
        name = "foo"
        version = "1.0.0"
    "#}));
    lockfile.assert(predicates::str::contains(indoc! {r#"
        [[package]]
        name = "bar"
        version = "1.0.0"
    "#}));
}
