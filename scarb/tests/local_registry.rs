use assert_fs::TempDir;
use assert_fs::prelude::*;
use indoc::{formatdoc, indoc};
use serde_json::json;
use url::Url;

use scarb_test_support::command::Scarb;
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
use scarb_test_support::registry::local::LocalRegistry;

#[test]
fn usage() {
    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("bar")
            .version("1.0.0")
            .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
            .build(t);
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("0.1.0")
        .dep("bar", Dep.version("1").registry(&registry))
        .lib_cairo(r#"fn f() -> felt252 { bar::f() }"#)
        .build(&t);

    // FIXME(mkaput): Why are verbose statuses not appearing here?
    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("");
}

#[test]
fn not_found() {
    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        // Publish a package so that the directory hierarchy is created.
        // Note, however, that we declare a dependency on baZ.
        ProjectBuilder::start()
            .name("bar")
            .version("1.0.0")
            .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
            .build(t);
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("0.1.0")
        .dep("baz", Dep.version("1").registry(&registry))
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: failed to lookup for `baz ^1 (registry+file://[..])` in registry: registry+file://[..]

        Caused by:
            0: failed to lookup for `baz ^1 (registry+file://[..])` in registry: registry+file://[..]
            1: package not found in registry: baz ^1 (registry+file://[..])
        "#});
}

// TODO(mkaput): Test interdependencies.
// TODO(mkaput): Test path dependencies overrides.

#[test]
fn empty_registry() {
    let registry = LocalRegistry::create();

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("0.1.0")
        .dep("baz", Dep.version("1").registry(&registry))
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: failed to lookup for `baz ^1 (registry+file://[..])` in registry: registry+file://[..]

        Caused by:
            0: failed to lookup for `baz ^1 (registry+file://[..])` in registry: registry+file://[..]
            1: package not found in registry: baz ^1 (registry+file://[..])
        "#});
}

#[test]
fn url_pointing_to_file() {
    let registry_t = TempDir::new().unwrap();
    let registry = registry_t.child("r");
    registry.write_str("").unwrap();
    let registry = Url::from_directory_path(&registry).unwrap().to_string();

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("0.1.0")
        .dep("baz", Dep.version("1").registry(&registry))
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: failed to load source: registry+file://[..]

        Caused by:
            0: failed to load source: registry+file://[..]
            1: local registry path is not a directory: [..]
        "#});

    // Prevent the temp directory from being deleted until this point.
    drop(registry_t);
}

#[test]
fn publish() {
    let t = TempDir::new().unwrap();
    let index = t.child("index");
    index.create_dir_all().unwrap();

    let make_and_publish = |name: &str, version: &str| {
        let t = TempDir::new().unwrap();
        ProjectBuilder::start()
            .name(name)
            .edition("2023_01")
            .version(version)
            .lib_cairo("fn main() -> felt252 { 0 }")
            .build(&t);

        Scarb::quick_snapbox()
            .arg("publish")
            .arg("--index")
            .arg(Url::from_directory_path(&index).unwrap().to_string())
            .current_dir(&t)
            .assert()
            .success()
            .stdout_matches(formatdoc! {r#"
            [..] Packaging {name} v{version} ([..])
            warn: manifest has no [..]
            warn: manifest has no [..]
            warn: manifest has no [..]
            warn: manifest has no [..]
            see [..] for more info
            [..]
            [..] Verifying {name}-{version}.tar.zst
            [..] Compiling {name} v{version} ([..])
            [..]  Finished `dev` profile target(s) in [..]
            [..]  Packaged [..]
            [..] Uploading {name} v{version} (registry+file://[..]/index/)
            [..] Published {name} v{version} (registry+file://[..]/index/)
            "#});
    };

    make_and_publish("hello", "1.0.0");
    make_and_publish("bar", "1.0.0");
    make_and_publish("hello", "1.1.0");

    assert_eq!(
        index.tree(),
        indoc! {r#"
            bar-1.0.0.tar.zst
            hello-1.0.0.tar.zst
            hello-1.1.0.tar.zst
            index/
            . 3/
            . . b/
            . . . bar.json
            . he/
            . . ll/
            . . . hello.json
        "#}
    );

    assert_eq!(
        index
            .child("index/3/b/bar.json")
            .assert_is_json::<serde_json::Value>(),
        json!([
            {
                "v": "1.0.0",
                "deps": [],
                "cksum": "sha256:04140f66ec3add19844f0be349797a5569c8637edff81f17e5e80be3bcfd2146",
            }
        ])
    );

    assert_eq!(
        index
            .child("index/he/ll/hello.json")
            .assert_is_json::<serde_json::Value>(),
        json!([
            {
                "v": "1.0.0",
                "deps": [],
                "cksum": "sha256:15c08d24286da6239a0bf56b1d68cd6dfa09761d1a43e6b2dfe807672b85c640",
            },
            {
                "v": "1.1.0",
                "deps": [],
                "cksum": "sha256:d9989ed9f02b62d464f1b641af51b9f409d546c597aa8418ccd208f453a75ba1",
            }
        ])
    );
}

#[test]
fn publish_disabled() {
    let t = TempDir::new().unwrap();
    let index = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("foobar")
        .version("1.0.0")
        .manifest_package_extra("publish = false")
        .lib_cairo("fn main() -> felt252 { 0 }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("publish")
        .arg("--no-verify")
        .arg("--index")
        .arg(Url::from_directory_path(&index).unwrap().to_string())
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: publishing disabled for package foobar v1.0.0 ([..]Scarb.toml)
        help: set `publish = true` in package manifest
        "#});
}

#[test]
fn publish_overwrites_existing() {
    let index = TempDir::new().unwrap();

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foobar")
        .edition("2023_01")
        .version("1.0.0")
        .lib_cairo("fn main() -> felt252 { 0 }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("publish")
        .arg("--no-verify")
        .arg("--index")
        .arg(Url::from_directory_path(&index).unwrap().to_string())
        .current_dir(&t)
        .assert()
        .success();

    assert_eq!(
        index
            .child("index/fo/ob/foobar.json")
            .assert_is_json::<serde_json::Value>(),
        json!([
            {
                "v": "1.0.0",
                "deps": [],
                "cksum": "sha256:03214b4a235dc6a1f439562758bee7c2abb9d876d0b2cc8f2211b570cc1b90c2",
            }
        ])
    );

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foobar")
        .edition("2023_01")
        .version("1.0.0")
        .lib_cairo("fn main() -> felt252 { 1024 }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("publish")
        .arg("--index")
        .arg(Url::from_directory_path(&index).unwrap().to_string())
        .current_dir(&t)
        .assert()
        .success();

    assert_eq!(
        index
            .child("index/fo/ob/foobar.json")
            .assert_is_json::<serde_json::Value>(),
        json!([
            {
                "v": "1.0.0",
                "deps": [],
                "cksum": "sha256:606fa349d05e8234b9aa614f1f0f5dc9f2c37fea8fde84f727af1f12a1305881",
            }
        ])
    );
}

// TODO(mkaput): Test errors properly when package is in index, but tarball is missing.
// TODO(mkaput): Test publishing with target-specific dependencies.
// TODO(mkaput): Test offline mode.
