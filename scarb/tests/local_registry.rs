use assert_fs::prelude::*;
use assert_fs::TempDir;
use indoc::{formatdoc, indoc};
use serde_json::json;
use url::Url;

use scarb_test_support::command::Scarb;
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
use scarb_test_support::registry::local::LocalRegistry;

// FIXME(#838)
#[test]
#[cfg_attr(target_os = "windows", ignore = "ignored on windows as of #838")]
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
        .stdout_matches(indoc! {r#"
        [..] Unpacking bar v1.0.0 ([..])
        "#});
}

// FIXME(#838)
#[test]
#[cfg_attr(target_os = "windows", ignore = "ignored on windows as of #838")]
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
        error: package not found in registry: baz ^1 (registry+file://[..])
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
        error: package not found in registry: baz ^1 (registry+file://[..])
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
            local registry path is not a directory: [..]
        "#});

    // Prevent the temp directory from being deleted until this point.
    drop(registry_t);
}

// FIXME(#838)
#[test]
#[cfg_attr(target_os = "windows", ignore = "ignored on windows as of #838")]
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
            [..]  Packaged [..]
            [..] Uploading {name} v{version} (registry+file://[..]/index/)
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
                "cksum": "sha256:d891504afc86fc0a7a9f38533a66ef2763990a1ff4be3eb9d5836d32a9bd9ad3",
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
                "cksum": "sha256:d05d4c524aa0136e42df6138f8e97f8b2b7fc946911cef8ae40baf38acf87ef6",
            },
            {
                "v": "1.1.0",
                "deps": [],
                "cksum": "sha256:ec55410dac39c63ea1372f44f05b74bcf14ec6305749d80bd607be0603271ef1",
            }
        ])
    );
}

// FIXME(#838)
#[test]
#[cfg_attr(target_os = "windows", ignore = "ignored on windows as of #838")]
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
                "cksum": "sha256:49bb7566594c89da4603578aebe812d750d1fefa1fccc532461963d813093b11",
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
                "cksum": "sha256:f6555b5b27327d40196578005de811158a3ac7401c36c13ee02b27afe7aab00f",
            }
        ])
    );
}

// TODO(mkaput): Test errors properly when package is in index, but tarball is missing.
// TODO(mkaput): Test publishing with target-specific dependencies.
// TODO(mkaput): Test offline mode.
