use std::fs;

use assert_fs::prelude::*;
use indoc::indoc;
use predicates::prelude::*;

use scarb::core::TomlManifest;

use crate::support::command::Scarb;
use crate::support::fsx::AssertFsUtf8Ext;

#[test]
fn new_simple() {
    let pt = assert_fs::TempDir::new().unwrap();

    Scarb::quick_snapbox()
        .arg("new")
        .arg("hello")
        .current_dir(&pt)
        .assert()
        .success();

    let t = pt.child("hello");
    assert!(t.is_dir());
    assert!(t.child("Scarb.toml").is_file());
    assert!(t.child("src/lib.cairo").is_file());
    assert!(t.child(".gitignore").is_file());
    assert!(t.child(".git").is_dir());

    let toml_manifest = TomlManifest::read_from_path(t.child("Scarb.toml").utf8_path()).unwrap();
    assert_eq!(toml_manifest.package.unwrap().name.as_str(), "hello");

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target/release/hello.sierra")
        .assert(predicates::str::is_empty().not());
}

#[test]
fn new_simple_without_vcs() {
    let pt = assert_fs::TempDir::new().unwrap();

    Scarb::quick_snapbox()
        .arg("new")
        .arg("hello")
        .arg("--no-vcs")
        .current_dir(&pt)
        .assert()
        .success();

    let t = pt.child("hello");
    assert!(t.is_dir());
    assert!(t.child("Scarb.toml").is_file());
    assert!(t.child("src/lib.cairo").is_file());
    assert!(!t.child(".gitignore").exists());
    assert!(!t.child(".git").exists());

    let toml_manifest = TomlManifest::read_from_path(t.child("Scarb.toml").utf8_path()).unwrap();
    assert_eq!(toml_manifest.package.unwrap().name.as_str(), "hello");

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target/release/hello.sierra")
        .assert(predicates::str::is_empty().not());
}

#[test]
fn init_simple() {
    let pt = assert_fs::TempDir::new().unwrap();
    let t = pt.child("hello");
    t.create_dir_all().unwrap();

    Scarb::quick_snapbox()
        .arg("init")
        .current_dir(&t)
        .assert()
        .success();

    assert!(t.is_dir());
    assert!(t.child("Scarb.toml").is_file());
    assert!(t.child("src/lib.cairo").is_file());
    assert!(t.child(".gitignore").is_file());
    assert!(t.child(".git").is_dir());

    let toml_manifest = TomlManifest::read_from_path(t.child("Scarb.toml").utf8_path()).unwrap();
    assert_eq!(toml_manifest.package.unwrap().name.as_str(), "hello");

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target/release/hello.sierra")
        .assert(predicates::str::is_empty().not());
}

#[test]
fn init_simple_without_vcs() {
    let pt = assert_fs::TempDir::new().unwrap();
    let t = pt.child("hello");
    t.create_dir_all().unwrap();

    Scarb::quick_snapbox()
        .arg("init")
        .arg("--no-vcs")
        .current_dir(&t)
        .assert()
        .success();

    assert!(t.is_dir());
    assert!(t.child("Scarb.toml").is_file());
    assert!(t.child("src/lib.cairo").is_file());
    assert!(!t.child(".gitignore").exists());
    assert!(!t.child(".git").exists());

    let toml_manifest = TomlManifest::read_from_path(t.child("Scarb.toml").utf8_path()).unwrap();
    assert_eq!(toml_manifest.package.unwrap().name.as_str(), "hello");

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target/release/hello.sierra")
        .assert(predicates::str::is_empty().not());
}

#[test]
fn new_no_path_arg() {
    Scarb::quick_snapbox()
        .arg("new")
        .assert()
        .failure()
        .stdout_eq("")
        .stderr_matches(indoc! {r#"
            error: the following required arguments were not provided:
              <PATH>

            Usage: scarb[..] new <PATH>

            For more information, try '--help'.
        "#});
}

#[test]
fn new_existing() {
    let pt = assert_fs::TempDir::new().unwrap();
    let t = pt.child("hello");
    t.create_dir_all().unwrap();

    Scarb::quick_snapbox()
        .arg("new")
        .arg("hello")
        .current_dir(&pt)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: destination `hello` already exists
            Use `scarb init` to initialize the directory.
        "#});
}

#[test]
fn invalid_package_name() {
    let pt = assert_fs::TempDir::new().unwrap();
    Scarb::quick_snapbox()
        .arg("new")
        .arg("a-b")
        .current_dir(&pt)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: invalid character `-` in package name: `a-b`, characters must be ASCII letter, ASCII numbers or underscore
        "#});
}

// TODO(mkaput): Test keyword as name.
// TODO(mkaput): Test core as name.

#[test]
fn new_explicit_project_name() {
    let pt = assert_fs::TempDir::new().unwrap();

    Scarb::quick_snapbox()
        .arg("new")
        .arg("hello")
        .arg("--name")
        .arg("world")
        .current_dir(&pt)
        .assert()
        .success();

    let t = pt.child("hello");

    let toml_manifest = TomlManifest::read_from_path(t.child("Scarb.toml").utf8_path()).unwrap();
    assert_eq!(toml_manifest.package.unwrap().name.as_str(), "world");
}

#[test]
fn init_existing_manifest() {
    let pt = assert_fs::TempDir::new().unwrap();
    let t = pt.child("hello");
    t.create_dir_all().unwrap();

    t.child("Scarb.toml").write_str("Scarb is great!").unwrap();

    Scarb::quick_snapbox()
        .arg("init")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: `scarb init` cannot be run on existing Scarb packages
        "#});
}

#[test]
fn init_existing_source() {
    let pt = assert_fs::TempDir::new().unwrap();
    let t = pt.child("hello");
    t.create_dir_all().unwrap();

    let src = t.child("src/lib.cairo");
    src.write_str("Scarb is great!").unwrap();

    Scarb::quick_snapbox()
        .arg("init")
        .current_dir(&t)
        .assert()
        .success();

    assert_eq!(fs::read_to_string(src).unwrap(), "Scarb is great!");
}

#[test]
fn init_does_not_overwrite_gitignore() {
    let pt = assert_fs::TempDir::new().unwrap();
    let t = pt.child("hello");
    t.create_dir_all().unwrap();
    t.child(".gitignore").write_str("examples\n").unwrap();

    Scarb::quick_snapbox()
        .arg("init")
        .current_dir(&t)
        .assert()
        .success();

    assert_eq!(
        &fs::read_to_string(t.child(".gitignore").path()).unwrap(),
        "examples\n"
    );
}
