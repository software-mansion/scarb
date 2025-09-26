use assert_fs::TempDir;
use assert_fs::prelude::*;
use indoc::indoc;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn assets_are_copied() {
    let t = TempDir::new().unwrap();

    t.child("data.txt")
        .write_str("“Marek, skup się na pracy.” ~ Marcin Skotniczny")
        .unwrap();

    ProjectBuilder::start()
        .name("foobar")
        .manifest_package_extra(r#"assets = ["data.txt"]"#)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target")
        .child("dev")
        .child("data.txt")
        .assert(predicates::path::is_file());
}

#[test]
fn asset_directory_is_error() {
    let t = TempDir::new().unwrap();

    t.child("assets").create_dir_all().unwrap();

    ProjectBuilder::start()
        .name("badpkg")
        .version("0.1.0")
        .manifest_package_extra(r#"assets = ["assets/"]"#)
        .build(&t);

    Scarb::quick_snapbox()
        .env("RUST_BACKTRACE", "0")
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(indoc! {r#"
            [..] Compiling badpkg v0.1.0 ([..])
            error: package `badpkg v0.1.0 ([..])` asset is not a file: [..]/assets
        "#});
}

#[test]
fn duplicate_asset_names_within_package_error() {
    let t = TempDir::new().unwrap();

    t.child("a").create_dir_all().unwrap();
    t.child("b").create_dir_all().unwrap();
    t.child("a/file.txt").write_str("A").unwrap();
    t.child("b/file.txt").write_str("B").unwrap();

    ProjectBuilder::start()
        .name("dupsame")
        .version("0.1.0")
        .manifest_package_extra(r#"assets = ["a/file.txt", "b/file.txt"]"#)
        .build(&t);

    Scarb::quick_snapbox()
        .env("RUST_BACKTRACE", "0")
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(indoc! {r#"
            [..] Compiling dupsame v0.1.0 ([..])
            error: package `dupsame v0.1.0 ([..])` declares multiple assets with the same file name: file.txt
        "#});
}

#[test]
fn missing_asset() {
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("missing")
        .version("0.1.0")
        .manifest_package_extra(r#"assets = ["data.txt"]"#)
        .build(&t);

    Scarb::quick_snapbox()
        .env("RUST_BACKTRACE", "0")
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(indoc! {r#"
            [..] Compiling missing v0.1.0 ([..])
            error: failed to find asset file at [..]/data.txt

            Caused by:
                0: failed to get absolute path of `[..]/data.txt`
                1: [..]
        "#});
}
