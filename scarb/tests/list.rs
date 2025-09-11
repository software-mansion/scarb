use assert_fs::prelude::PathChild;
use indoc::{formatdoc, indoc};
use scarb_build_metadata::CAIRO_VERSION;
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

    yank(registry.t.child("index/3/f/foo.json").path(), "1.2.4-beta").unwrap();
    audit(registry.t.child("index/3/f/foo.json").path(), "1.5.0").unwrap();

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
            1.5.0            ✓        -[..]
            1.2.4-beta       x        yanked[..]
            1.2.3            x        -
        "#});

    Scarb::quick_snapbox()
        .arg("--json")
        .arg("list")
        .arg("foo")
        .arg("--index")
        .arg(&registry.url)
        .assert()
        .success()
        .stdout_matches(
            indoc!{
            r#"
            [{"v":"2.0.0+build.1","deps":[],"cksum":"sha256:[..]"},{"v":"2.0.0-alpha.1","deps":[],"cksum":"sha256:[..]"},{"v":"1.5.0","deps":[],"cksum":"sha256:[..]","audited":true},{"v":"1.2.4-beta","deps":[],"cksum":"sha256:[..]","yanked":true},{"v":"1.2.3","deps":[],"cksum":"sha256:[..]"}]
            "#
            }
        );
}

#[test]
fn list_builtin_package_versions() {
    let mut registry = LocalRegistry::create();
    for version in &["0.1.0", "0.1.1", "0.1.2"] {
        publish_package("starknet", version, &mut registry);
    }

    Scarb::quick_snapbox()
        .arg("list")
        .arg("starknet")
        .arg("--index")
        .arg(&registry.url)
        .assert()
        .success()
        .stdout_matches(formatdoc! {r#"
            warn: the package `starknet` is a part of Cairo standard library.
            its available version ({CAIRO_VERSION}) is coupled to the Cairo version included in your Scarb installation.
            help: to use another version of this package, consider using a different version of Scarb.

            VERSION    AUDIT    STATUS
            0.1.2      x        -[..]
            0.1.1      x        -[..]
            0.1.0      x        -
        "#});

    Scarb::quick_snapbox()
        .arg("--json")
        .arg("list")
        .arg("starknet")
        .arg("--index")
        .arg(&registry.url)
        .assert()
        .success()
        .stdout_matches(
            formatdoc!{
            r#"
            {{"type":"warn","message":"the package `starknet` is a part of Cairo standard library./nits available version ({CAIRO_VERSION}) is coupled to the Cairo version included in your Scarb installation./nhelp: to use another version of this package, consider using a different version of Scarb./n"}}
            [{{"v":"0.1.2","deps":[],"cksum":"sha256:[..]"}},{{"v":"0.1.1","deps":[],"cksum":"sha256:[..]"}},{{"v":"0.1.0","deps":[],"cksum":"sha256:[..]"}}]
            "#
            }
        );
}

#[test]
fn list_package_versions_many() {
    let mut registry = LocalRegistry::create();
    let versions = vec!["0.1.0", "0.2.0", "0.3.0", "0.4.0", "0.5.0", "0.6.0"];
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
            VERSION    AUDIT    STATUS
            0.6.0      x        -[..]
            0.5.0      x        -[..]
            0.4.0      x        -[..]
            0.3.0      x        -[..]
            0.2.0      x        -[..]
            ...
            use `--all` or `--limit 6` to show all 6 versions
        "#});

    Scarb::quick_snapbox()
        .arg("list")
        .arg("foo")
        .arg("--all")
        .arg("--index")
        .arg(&registry.url)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            VERSION    AUDIT    STATUS
            0.6.0      x        -[..]
            0.5.0      x        -[..]
            0.4.0      x        -[..]
            0.3.0      x        -[..]
            0.2.0      x        -[..]
            0.1.0      x        -
        "#});

    Scarb::quick_snapbox()
        .arg("list")
        .arg("foo")
        .arg("--limit")
        .arg("3")
        .arg("--index")
        .arg(&registry.url)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            VERSION    AUDIT    STATUS
            0.6.0      x        -[..]
            0.5.0      x        -[..]
            0.4.0      x        -[..]
            ...
            use `--all` or `--limit 6` to show all 6 versions
        "#});

    Scarb::quick_snapbox()
        .arg("list")
        .arg("foo")
        .arg("--limit")
        .arg("100")
        .arg("--index")
        .arg(&registry.url)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            VERSION    AUDIT    STATUS
            0.6.0      x        -[..]
            0.5.0      x        -[..]
            0.4.0      x        -[..]
            0.3.0      x        -[..]
            0.2.0      x        -[..]
            0.1.0      x        -
        "#});
}
