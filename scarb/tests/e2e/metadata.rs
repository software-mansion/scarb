use assert_fs::prelude::*;

use crate::support::command::Scarb;
use crate::support::project_builder::ProjectBuilder;

#[test]
fn simple() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .build(&t);

    Scarb::quick_snapbox()
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
      "id": "core 0.1.0 (core+https://github.com/starkware-libs/cairo.git)",
      "name": "core",
      "version": "0.1.0",
      "source": "core+https://github.com/starkware-libs/cairo.git",
      "root": "[..]",
      "manifest_path": "[..]/Scarb.toml",
      "dependencies": [],
      "targets": [
        {
          "kind": "lib",
          "name": "core",
          "params": {
            "casm": false,
            "sierra": true
          }
        }
      ],
      "authors": null,
      "urls": null,
      "metadata": null,
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
      "id": "hello 0.1.0 (path+file://[..])",
      "name": "hello",
      "version": "0.1.0",
      "source": "path+file://[..]",
      "root": "[..]",
      "manifest_path": "[..]/Scarb.toml",
      "dependencies": [
        {
          "name": "core",
          "version_req": "*",
          "source": "core+https://github.com/starkware-libs/cairo.git"
        }
      ],
      "targets": [
        {
          "kind": "lib",
          "name": "hello",
          "params": {
            "casm": false,
            "sierra": true
          }
        }
      ],
      "authors": null,
      "urls": null,
      "metadata": null,
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
    ProjectBuilder::start().build(&t);

    Scarb::quick_snapbox()
        .arg("metadata")
        .current_dir(&t)
        .assert()
        .failure();
}

fn create_local_dependencies_setup(t: &assert_fs::TempDir) {
    t.child("Scarb.toml")
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

    t.child("y/Scarb.toml")
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

    t.child("z/Scarb.toml")
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

    t.child("q/Scarb.toml")
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
    Scarb::quick_snapbox()
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
      "id": "core 0.1.0 (core+https://github.com/starkware-libs/cairo.git)",
      "name": "core",
      "version": "0.1.0",
      "source": "core+https://github.com/starkware-libs/cairo.git",
      "root": "[..]",
      "manifest_path": "[..]/Scarb.toml",
      "dependencies": [],
      "targets": [
        {
          "kind": "lib",
          "name": "core",
          "params": {
            "casm": false,
            "sierra": true
          }
        }
      ],
      "authors": null,
      "urls": null,
      "metadata": null,
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
      "id": "q 1.0.0 (path+file://[..]/q/)",
      "name": "q",
      "version": "1.0.0",
      "source": "path+file://[..]",
      "root": "[..]",
      "manifest_path": "[..]/Scarb.toml",
      "dependencies": [
        {
          "name": "core",
          "version_req": "*",
          "source": "core+https://github.com/starkware-libs/cairo.git"
        }
      ],
      "targets": [
        {
          "kind": "lib",
          "name": "q",
          "params": {
            "casm": false,
            "sierra": true
          }
        }
      ],
      "authors": null,
      "urls": null,
      "metadata": null,
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
      "manifest_path": "[..]/Scarb.toml",
      "dependencies": [
        {
          "name": "core",
          "version_req": "*",
          "source": "core+https://github.com/starkware-libs/cairo.git"
        },
        {
          "name": "y",
          "version_req": "*",
          "source": "path+file://[..]/y/"
        }
      ],
      "targets": [
        {
          "kind": "lib",
          "name": "x",
          "params": {
            "casm": false,
            "sierra": true
          }
        }
      ],
      "authors": null,
      "urls": null,
      "metadata": null,
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
      "manifest_path": "[..]/Scarb.toml",
      "dependencies": [
        {
          "name": "core",
          "version_req": "*",
          "source": "core+https://github.com/starkware-libs/cairo.git"
        },
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
      "targets": [
        {
          "kind": "lib",
          "name": "y",
          "params": {
            "casm": false,
            "sierra": true
          }
        }
      ],
      "authors": null,
      "urls": null,
      "metadata": null,
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
      "manifest_path": "[..]/Scarb.toml",
      "dependencies": [
        {
          "name": "core",
          "version_req": "*",
          "source": "core+https://github.com/starkware-libs/cairo.git"
        },
        {
          "name": "q",
          "version_req": "*",
          "source": "path+file://[..]/q/"
        }
      ],
      "targets": [
        {
          "kind": "lib",
          "name": "z",
          "params": {
            "casm": false,
            "sierra": true
          }
        }
      ],
      "authors": null,
      "urls": null,
      "metadata": null,
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
    Scarb::quick_snapbox()
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
      "manifest_path": "[..]/Scarb.toml",
      "dependencies": [
        {
          "name": "core",
          "version_req": "*",
          "source": "core+https://github.com/starkware-libs/cairo.git"
        },
        {
          "name": "y",
          "version_req": "*",
          "source": "path+file://[..]/y/"
        }
      ],
      "targets": [
        {
          "kind": "lib",
          "name": "x",
          "params": {
            "casm": false,
            "sierra": true
          }
        }
      ],
      "authors": null,
      "urls": null,
      "metadata": null,
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
fn manifest_targets_and_metadata() {
    let t = assert_fs::TempDir::new().unwrap();
    t.child("Scarb.toml")
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

            [package.urls]
            hello = "https://world.com/"

            [package.metadata]
            meta = "data"
            numeric = "1231"
            key = "value"

            [lib]
            sierra = false
            casm = true

            [[target.example]]
            string = "bar"
            number = 1234
            bool = true
            array = ["a", 1]
            table = { x = "y" }
            "#,
        )
        .unwrap();

    Scarb::quick_snapbox()
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
      "id": "core 0.1.0 (core+https://github.com/starkware-libs/cairo.git)",
      "name": "core",
      "version": "0.1.0",
      "source": "core+https://github.com/starkware-libs/cairo.git",
      "root": "[..]",
      "manifest_path": "[..]/Scarb.toml",
      "dependencies": [],
      "targets": [
        {
          "kind": "lib",
          "name": "core",
          "params": {
            "casm": false,
            "sierra": true
          }
        }
      ],
      "authors": null,
      "urls": null,
      "metadata": null,
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
      "id": "hello 0.1.0 (path+file://[..])",
      "name": "hello",
      "version": "0.1.0",
      "source": "path+file://[..]",
      "root": "[..]",
      "manifest_path": "[..]/Scarb.toml",
      "dependencies": [
        {
          "name": "core",
          "version_req": "*",
          "source": "core+https://github.com/starkware-libs/cairo.git"
        }
      ],
      "targets": [
        {
          "kind": "example",
          "name": "hello",
          "params": {
            "array": [
              "a",
              1
            ],
            "bool": true,
            "number": 1234,
            "string": "bar",
            "table": {
              "x": "y"
            }
          }
        },
        {
          "kind": "lib",
          "name": "hello",
          "params": {
            "casm": true,
            "sierra": false
          }
        }
      ],
      "authors": [
        "John Doe <john.doe@swmansion.com>",
        "Jane Doe <jane.doe@swmansion.com>"
      ],
      "urls": {
        "hello": "https://world.com/"
      },
      "metadata": {
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

#[test]
fn json_output_is_not_pretty() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches("{\"version\":1,[..]}\n");
}

// TODO(mkaput): Add tests with workspaces
