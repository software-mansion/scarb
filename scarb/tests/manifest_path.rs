use assert_fs::prelude::*;
use snapbox::Data;

use scarb_test_support::command::Scarb;

#[test]
fn with_manifest() {
    let t = assert_fs::TempDir::new().unwrap();
    let manifest = t.child("Scarb.toml");
    manifest
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();

    Scarb::quick_command()
        .arg("manifest-path")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(
            Data::from(format!(
                "{}\n",
                dunce::canonicalize(manifest.path()).unwrap().display()
            ))
            .raw(),
        );
}

#[test]
#[cfg_attr(
    target_os = "windows",
    ignore = "This test does not properly deal with short (8.3) paths. \
    This is not a problem in other tests, because they properly canonicalize paths for output, \
    as these paths do exist."
)]
fn without_manifest() {
    let t = assert_fs::TempDir::new().unwrap();

    Scarb::quick_command()
        .arg("manifest-path")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(
            Data::from(format!(
                "{}\n",
                dunce::canonicalize(t.path())
                    .unwrap()
                    .join("Scarb.toml")
                    .display()
            ))
            .raw(),
        );
}

#[test]
fn subdir() {
    let t = assert_fs::TempDir::new().unwrap();
    let manifest = t.child("Scarb.toml");
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

    Scarb::quick_command()
        .arg("manifest-path")
        .current_dir(&subdir)
        .assert()
        .success()
        .stdout_eq(
            Data::from(format!(
                "{}\n",
                dunce::canonicalize(manifest.path()).unwrap().display()
            ))
            .raw(),
        );
}

#[test]
fn path_override() {
    let t = assert_fs::TempDir::new().unwrap();

    let subdir = t.child("foobar");
    subdir.create_dir_all().unwrap();

    let manifest = subdir.child("Scarb.toml");
    manifest
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();

    Scarb::quick_command()
        .arg("--manifest-path")
        .arg(manifest.path())
        .arg("manifest-path")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(
            Data::from(format!(
                "{}\n",
                dunce::canonicalize(manifest.path()).unwrap().display()
            ))
            .raw(),
        );
}

#[test]
fn path_override_no_manifest() {
    let t = assert_fs::TempDir::new().unwrap();

    let subdir = t.child("foobar");
    subdir.create_dir_all().unwrap();

    let manifest = subdir.child("Scarb.toml");

    Scarb::quick_command()
        .arg("--manifest-path")
        .arg(manifest.path())
        .arg("manifest-path")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(Data::from(format!("{}\n", manifest.path().display())).raw());
}

#[test]
fn path_override_via_env() {
    let t = assert_fs::TempDir::new().unwrap();

    let subdir = t.child("foobar");
    subdir.create_dir_all().unwrap();

    let manifest = subdir.child("Scarb.toml");
    manifest
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();

    Scarb::quick_command()
        .env("SCARB_MANIFEST_PATH", manifest.path())
        .arg("manifest-path")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(
            Data::from(format!(
                "{}\n",
                dunce::canonicalize(manifest.path()).unwrap().display()
            ))
            .raw(),
        );
}

#[test]
fn json_output() {
    let t = assert_fs::TempDir::new().unwrap();
    let manifest = t.child("Scarb.toml");
    manifest
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();

    Scarb::quick_command()
        .arg("--json")
        .arg("manifest-path")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("{\"path\":\"[..]\"}\n");
}
