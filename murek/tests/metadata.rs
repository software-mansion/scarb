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
    "root": "[..]",
    "members": [
      "hello 0.1.0 (path+file://[..])"
    ]
  },
  "packages": [
    {
      "id": "hello 0.1.0 (path+file://[..])",
      "name": "hello",
      "version": "0.1.0",
      "source": "path+file://[..]",
      "root": "[..]",
      "manifest_path": "[..]/Murek.toml",
      "dependencies": [],
      "authors": null,
      "custom_links": null,
      "custom_metadata": null,
      "description": null,
      "documentation": null,
      "homepage": null,
      "keywords": null,
      "license": null,
      "license_file": null,
      "readme": null,
      "repository": null
    }
  ]
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

fn create_local_dependencies_setup(t: &assert_fs::TempDir) {
    t.child("Murek.toml")
        .write_str(
            r#"
            [package]
            name = "x"
            version = "1.0.0"

            [dependencies]
            y = { path = "y" }
            "#,
        )
        .unwrap();

    t.child("src/lib.cairo")
        .write_str(r"fn f() -> felt { y::f() }")
        .unwrap();

    t.child("y/Murek.toml")
        .write_str(
            r#"
            [package]
            name = "y"
            version = "1.0.0"

            [dependencies]
            q = { path = "../q" }
            z = { path = "../z" }
            "#,
        )
        .unwrap();

    t.child("y/src/lib.cairo")
        .write_str(r"fn f() -> felt { z::f() }")
        .unwrap();

    t.child("z/Murek.toml")
        .write_str(
            r#"
            [package]
            name = "z"
            version = "1.0.0"

            [dependencies]
            q = { path = "../q" }
            "#,
        )
        .unwrap();

    t.child("z/src/lib.cairo")
        .write_str(r"fn f() -> felt { q::f() }")
        .unwrap();

    t.child("q/Murek.toml")
        .write_str(
            r#"
            [package]
            name = "q"
            version = "1.0.0"
            "#,
        )
        .unwrap();

    t.child("q/src/lib.cairo")
        .write_str(r"fn f() -> felt { 42 }")
        .unwrap();
}

#[test]
fn local_dependencies() {
    let t = assert_fs::TempDir::new().unwrap();
    create_local_dependencies_setup(&t);
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
    "root": "[..]",
    "members": [
      "x 1.0.0 (path+file://[..])"
    ]
  },
  "packages": [
    {
      "id": "q 1.0.0 (path+file://[..]/q/)",
      "name": "q",
      "version": "1.0.0",
      "source": "path+file://[..]",
      "root": "[..]",
      "manifest_path": "[..]/Murek.toml",
      "dependencies": [],
      "authors": null,
      "custom_links": null,
      "custom_metadata": null,
      "description": null,
      "documentation": null,
      "homepage": null,
      "keywords": null,
      "license": null,
      "license_file": null,
      "readme": null,
      "repository": null
    },
    {
      "id": "x 1.0.0 (path+file://[..])",
      "name": "x",
      "version": "1.0.0",
      "source": "path+file://[..]",
      "root": "[..]",
      "manifest_path": "[..]/Murek.toml",
      "dependencies": [
        {
          "name": "y",
          "version_req": "*",
          "source": "path+file://[..]/y/"
        }
      ],
      "authors": null,
      "custom_links": null,
      "custom_metadata": null,
      "description": null,
      "documentation": null,
      "homepage": null,
      "keywords": null,
      "license": null,
      "license_file": null,
      "readme": null,
      "repository": null
    },
    {
      "id": "y 1.0.0 (path+file://[..]/y/)",
      "name": "y",
      "version": "1.0.0",
      "source": "path+file://[..]",
      "root": "[..]",
      "manifest_path": "[..]/Murek.toml",
      "dependencies": [
        {
          "name": "q",
          "version_req": "*",
          "source": "path+file://[..]/q/"
        },
        {
          "name": "z",
          "version_req": "*",
          "source": "path+file://[..]/z/"
        }
      ],
      "authors": null,
      "custom_links": null,
      "custom_metadata": null,
      "description": null,
      "documentation": null,
      "homepage": null,
      "keywords": null,
      "license": null,
      "license_file": null,
      "readme": null,
      "repository": null
    },
    {
      "id": "z 1.0.0 (path+file://[..]/z/)",
      "name": "z",
      "version": "1.0.0",
      "source": "path+file://[..]",
      "root": "[..]",
      "manifest_path": "[..]/Murek.toml",
      "dependencies": [
        {
          "name": "q",
          "version_req": "*",
          "source": "path+file://[..]/q/"
        }
      ],
      "authors": null,
      "custom_links": null,
      "custom_metadata": null,
      "description": null,
      "documentation": null,
      "homepage": null,
      "keywords": null,
      "license": null,
      "license_file": null,
      "readme": null,
      "repository": null
    }
  ]
}
"#,
        );
}

#[test]
fn no_dep() {
    let t = assert_fs::TempDir::new().unwrap();
    create_local_dependencies_setup(&t);
    Command::new(cargo_bin!("murek"))
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--no-deps")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(
            r#"{
  "version": 1,
  "app_exe": "[..]",
  "target_dir": "[..]/target",
  "workspace": {
    "root": "[..]",
    "members": [
      "x 1.0.0 (path+file://[..])"
    ]
  },
  "packages": [
    {
      "id": "x 1.0.0 (path+file://[..])",
      "name": "x",
      "version": "1.0.0",
      "source": "path+file://[..]",
      "root": "[..]",
      "manifest_path": "[..]/Murek.toml",
      "dependencies": [
        {
          "name": "y",
          "version_req": "*",
          "source": "path+file://[..]/y/"
        }
      ],
      "authors": null,
      "custom_links": null,
      "custom_metadata": null,
      "description": null,
      "documentation": null,
      "homepage": null,
      "keywords": null,
      "license": null,
      "license_file": null,
      "readme": null,
      "repository": null
    }
  ]
}
"#,
        );
}

#[test]
fn manifest_metadata() {
    let t = assert_fs::TempDir::new().unwrap();
    t.child("Murek.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"

            description = "Some interesting description to read!"
            authors = ["John Doe <john.doe@swmansion.com>", "Jane Doe <jane.doe@swmansion.com>"]
            keywords = ["some", "project", "keywords"]

            homepage = "http://www.homepage.com/"
            documentation = "http://docs.homepage.com/"
            repository = "http://github.com/johndoe/repo"

            license = "MIT License"
            license-file = "./license.md"
            readme = "./readme.md"

            [package.custom-links]
            hello = "https://world.com/"

            [package.custom-metadata]
            meta = "data"
            numeric = "1231"
            key = "value"
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
    "root": "[..]",
    "members": [
      "hello 0.1.0 (path+file://[..])"
    ]
  },
  "packages": [
    {
      "id": "hello 0.1.0 (path+file://[..])",
      "name": "hello",
      "version": "0.1.0",
      "source": "path+file://[..]",
      "root": "[..]",
      "manifest_path": "[..]/Murek.toml",
      "dependencies": [],
      "authors": [
        "John Doe <john.doe@swmansion.com>",
        "Jane Doe <jane.doe@swmansion.com>"
      ],
      "custom_links": {
        "hello": "https://world.com/"
      },
      "custom_metadata": {
        "key": "value",
        "meta": "data",
        "numeric": "1231"
      },
      "description": "Some interesting description to read!",
      "documentation": "http://docs.homepage.com/",
      "homepage": "http://www.homepage.com/",
      "keywords": [
        "some",
        "project",
        "keywords"
      ],
      "license": "MIT License",
      "license_file": "./license.md",
      "readme": "./readme.md",
      "repository": "http://github.com/johndoe/repo"
    }
  ]
}
"#,
        );
}

// TODO(mkaput): Add tests with workspaces
