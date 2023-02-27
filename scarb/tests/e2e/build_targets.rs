use assert_fs::prelude::*;
use assert_fs::TempDir;
use indoc::indoc;
use predicates::prelude::*;

use crate::support::command::Scarb;

#[test]
fn compile_with_duplicate_targets_1() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"

            [[target.example]]

            [[target.example]]
            "#,
        )
        .unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: failed to parse manifest at `[..]/Scarb.toml`

        Caused by:
            manifest contains duplicate target definitions `example`, consider explicitly naming targets with the `name` field
        "#});
}

#[test]
fn compile_with_duplicate_targets_2() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"

            [[target.example]]
            name = "x"

            [[target.example]]
            name = "x"
            "#,
        )
        .unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: failed to parse manifest at `[..]/Scarb.toml`

        Caused by:
            manifest contains duplicate target definitions `example (x)`, use different target names to resolve the conflict
        "#});
}

#[test]
fn compile_with_custom_lib_target() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"

            [lib]
            name = "not_hello"
            sierra = false
            casm = true
            "#,
        )
        .unwrap();
    t.child("src/lib.cairo")
        .write_str(r#"fn f() -> felt { 42 }"#)
        .unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        [..]  Finished release target(s) in [..]
        "#});

    t.child("target/release/not_hello.casm")
        .assert(predicates::str::is_empty().not());
    t.child("target/release/not_hello.sierra")
        .assert(predicates::path::exists().not());
    t.child("target/release/hello.sierra")
        .assert(predicates::path::exists().not());
    t.child("target/release/hello.casm")
        .assert(predicates::path::exists().not());
}
