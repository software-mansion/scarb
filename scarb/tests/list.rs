use assert_fs::TempDir;
use assert_fs::prelude::PathChild;
use indoc::indoc;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;
use scarb_test_support::registry::local::{LocalRegistry, audit, yank};

fn publish_package(name: &str, version: &str, registry: &mut LocalRegistry) {
    registry.publish(|t| {
        ProjectBuilder::start()
            .name(name)
            .version(version)
            .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
            .build(t);
    });
}

#[test]
fn list_package_versions() {
    let mut registry = LocalRegistry::create();
    let versions = vec![
        "1.5.0",
        "1.2.3",
        "2.0.0+build.1",
        "2.0.0-alpha.1",
        "1.2.4-beta",
    ];
    for version in &versions {
        publish_package("foo", version, &mut registry);
    }

    Scarb::quick_snapbox()
        .arg("list")
        .arg("foo")
        .arg("--index")
        .arg(&registry.url)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            VERSION          AUDIT    STATUS
            2.0.0+build.1    x        -[..]
            2.0.0-alpha.1    x        -[..]
            1.5.0            x        -[..]
            1.2.4-beta       x        -[..]
            1.2.3            x        -
        "#});
}

fn list_package_versions_yanked() {
    let mut registry = LocalRegistry::create();
    let versions = vec!["1.0.0", "1.1.0", "2.0.0"];
    for version in &versions {
        publish_package("foo", version, &mut registry);
    }
    yank(registry.t.child("index/3/f/foo.json").path(), "1.1.0").unwrap();

    Scarb::quick_snapbox()
        .arg("list")
        .arg("foo")
        .arg("--index")
        .arg(&registry.url)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            VERSION    AUDIT    STATUS
            2.0.0      x        -[..]
            1.1.0      x        yanked[..]
            1.0.0      x        -
        "#});
}

#[test]
fn list_package_versions_audited() {
    let mut registry = LocalRegistry::create();
    let versions = vec!["1.0.0", "1.1.0", "2.0.0"];
    for version in &versions {
        publish_package("foo", version, &mut registry);
    }
    audit(registry.t.child("index/3/f/foo.json").path(), "1.1.0").unwrap();

    Scarb::quick_snapbox()
        .arg("list")
        .arg("foo")
        .arg("--index")
        .arg(&registry.url)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            VERSION    AUDIT    STATUS
            2.0.0      x        -[..]
            1.1.0      ✓        -[..]
            1.0.0      x        -
        "#});
}
