use assert_fs::prelude::*;
use assert_fs::TempDir;
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
        error: cannot get dependencies of `foo@0.1.0`

        Caused by:
            0: failed to lookup for `baz ^1 (registry+file://[..])` in registry: registry+file://[..]
            1: failed to lookup for `baz ^1 (registry+file://[..])` in registry: registry+file://[..]
            2: package not found in registry: baz ^1 (registry+file://[..])
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
        error: cannot get dependencies of `foo@0.1.0`

        Caused by:
            0: failed to lookup for `baz ^1 (registry+file://[..])` in registry: registry+file://[..]
            1: failed to lookup for `baz ^1 (registry+file://[..])` in registry: registry+file://[..]
            2: package not found in registry: baz ^1 (registry+file://[..])
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
        error: cannot get dependencies of `foo@0.1.0`

        Caused by:
            0: failed to load source: registry+file://[..]
            1: failed to load source: registry+file://[..]
            2: local registry path is not a directory: [..]
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
                "cksum": "sha256:3771f51595f7df697a822823989714b3c9a83764a67fa517ac6d8df3cfc642bf",
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
                "cksum": "sha256:07db55042bb0be41eb957d649960cd16764426ce9df1cd0729d408acdf3c63cb",
            },
            {
                "v": "1.1.0",
                "deps": [],
                "cksum": "sha256:bd36828fa2134ec3ce9599511404f85958d8ed091fc3b03a3f3d8426d0a0f720",
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
                "cksum": "sha256:b34e1202407e1a9b743f261cdc27723d0344619a6dc3058bdacd9b17f6106027",
            }
        ])
    );

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foobar")
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
                "cksum": "sha256:4682a7645acf0b342556778a0544e2871ae910efc609d89ab3bedf9491219ab5",
            }
        ])
    );
}

// TODO(mkaput): Test errors properly when package is in index, but tarball is missing.
// TODO(mkaput): Test publishing with target-specific dependencies.
// TODO(mkaput): Test offline mode.
