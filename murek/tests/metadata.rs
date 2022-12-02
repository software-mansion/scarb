use assert_fs::prelude::*;
use snapbox::cmd::{cargo_bin, Command};

#[test]
fn simple() {
    let t = assert_fs::TempDir::new().unwrap();
    t.child("Murek.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();

    Command::new(cargo_bin!("murek"))
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(
            r#"{
  "version": 1,
  "app_exe": "[..]",
  "target_dir": "[..]/target",
  "workspace": {
    "workspace_root": "[..]",
    "members": [
      "hello 0.1.0 (path+file://[..])"
    ],
    "packages": [
      {
        "name": "hello",
        "version": "0.1.0",
        "id": "hello 0.1.0 (path+file://[..])",
        "source": "path+file://[..]",
        "root": "[..]",
        "manifest_path": "[..]/Murek.toml",
        "dependencies": []
      }
    ]
  }
}
"#,
        );
}

#[test]
fn fails_without_format_version() {
    let t = assert_fs::TempDir::new().unwrap();
    t.child("Murek.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();

    Command::new(cargo_bin!("murek"))
        .arg("metadata")
        .current_dir(&t)
        .assert()
        .failure();
}

// TODO(mkaput): Add tests with dependencies
// TODO(mkaput): Add tests with --no-dep
// TODO(mkaput): Add tests with workspaces
