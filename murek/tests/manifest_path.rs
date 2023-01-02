use std::fs;

use assert_fs::prelude::*;
use snapbox::cmd::{cargo_bin, Command};

#[test]
fn with_manifest() {
    let t = assert_fs::TempDir::new().unwrap();
    let manifest = t.child("Murek.toml");
    manifest
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();

    Command::new(cargo_bin!("murek"))
        .arg("manifest-path")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(format!(
            "{}\n",
            fs::canonicalize(manifest.path()).unwrap().display()
        ));
}

#[test]
fn without_manifest() {
    let t = assert_fs::TempDir::new().unwrap();

    Command::new(cargo_bin!("murek"))
        .arg("manifest-path")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(format!(
            "{}\n",
            fs::canonicalize(t.path())
                .unwrap()
                .join("Murek.toml")
                .display()
        ));
}

#[test]
fn subdir() {
    let t = assert_fs::TempDir::new().unwrap();
    let manifest = t.child("Murek.toml");
    manifest
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();

    let subdir = t.child("foobar");
    subdir.create_dir_all().unwrap();

    Command::new(cargo_bin!("murek"))
        .arg("manifest-path")
        .current_dir(&subdir)
        .assert()
        .success()
        .stdout_eq(format!(
            "{}\n",
            fs::canonicalize(manifest.path()).unwrap().display()
        ));
}

#[test]
fn path_override() {
    let t = assert_fs::TempDir::new().unwrap();

    let subdir = t.child("foobar");
    subdir.create_dir_all().unwrap();

    let manifest = subdir.child("Murek.toml");
    manifest
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();

    Command::new(cargo_bin!("murek"))
        .arg("--manifest-path")
        .arg(manifest.path())
        .arg("manifest-path")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(format!(
            "{}\n",
            fs::canonicalize(manifest.path()).unwrap().display()
        ));
}

#[test]
fn path_override_no_manifest() {
    let t = assert_fs::TempDir::new().unwrap();

    let subdir = t.child("foobar");
    subdir.create_dir_all().unwrap();

    let manifest = subdir.child("Murek.toml");

    Command::new(cargo_bin!("murek"))
        .arg("--manifest-path")
        .arg(manifest.path())
        .arg("manifest-path")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(format!("{}\n", manifest.path().display()));
}

#[test]
fn path_override_via_env() {
    let t = assert_fs::TempDir::new().unwrap();

    let subdir = t.child("foobar");
    subdir.create_dir_all().unwrap();

    let manifest = subdir.child("Murek.toml");
    manifest
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();

    Command::new(cargo_bin!("murek"))
        .env("MUREK_MANIFEST_PATH", manifest.path())
        .arg("manifest-path")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(format!(
            "{}\n",
            fs::canonicalize(manifest.path()).unwrap().display()
        ));
}
