use assert_fs::prelude::*;
use assert_fs::TempDir;
use indoc::{formatdoc, indoc};
use serde_json::json;
use url::Url;

use scarb_test_support::command::Scarb;
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::project_builder::ProjectBuilder;

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
                "cksum": "sha256:13973a8c7a6d86430ad569fd2c2d5cad282ba67ee587820a4b597f7b0a66a8dd",
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
                "cksum": "sha256:032b626571a86bb18d93d6e67376d5c9b5a14efd76871bb5e3de4b1ded3c6c64",
            },
            {
                "v": "1.1.0",
                "deps": [],
                "cksum": "sha256:0b9c792212d383b00b3b059461caa1bea64b1528890d54f95ea678d2956ec613",
            }
        ])
    );
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
                "cksum": "sha256:d3356ff99d397d9963f88318b4c0019b61037255a9a632cc1fe24b9aa876a607",
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
                "cksum": "sha256:207317e685713fcda79fa2172b5d3ca8d138efc7cee3c6c0960a17ba980738bd",
            }
        ])
    );
}

// TODO(mkaput): Test publishing with target-specific dependencies.
