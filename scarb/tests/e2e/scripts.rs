use std::collections::BTreeMap;
use std::env;

use assert_fs::prelude::*;
use assert_fs::TempDir;
use indoc::{formatdoc, indoc};
use snapbox::cmd::Command;

use scarb::process::make_executable;

use crate::support::command::Scarb;
use crate::support::filesystem::{path_with_temp_dir, write_script};
use crate::support::project_builder::ProjectBuilder;

trait CommandExt {
    fn stdout_json(self) -> BTreeMap<String, String>;
}

impl CommandExt for Command {
    fn stdout_json(self) -> BTreeMap<String, String> {
        let output = self.output().expect("Failed to spawn command");
        assert!(
            output.status.success(),
            "Command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::de::from_slice(&output.stdout).expect("Failed to deserialize stdout to JSON")
    }
}

#[test]
fn run_simple_script() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .manifest_extra(indoc! {r#"
        [scripts]
        some_script = "echo 'Hello, world!'"
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .args(["run", "some_script"])
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("Hello, world!\n");
}

#[test]
fn run_missing_script() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .manifest_extra(indoc! {r#"
        [scripts]
        some_script = "echo 'Hello, world!'"
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .args(["run", "some_other_script"])
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: missing script `some_other_script`

            To see a list of scripts, run:
                scarb run
        "#});
}

#[test]
fn list_scripts() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .manifest_extra(indoc! {r#"
        [scripts]
        some_script = "echo 'Hello'"
        some_other_script = "echo 'world!'"
        "#})
        .build(&t);

    let output = Scarb::quick_snapbox()
        .args(["--json", "run"])
        .current_dir(&t)
        .stdout_json();

    assert_eq!(output["some_script"], "echo 'Hello'");
    assert_eq!(output["some_other_script"], "echo 'world!'");
    assert_eq!(output.len(), 2);
}

#[test]
fn list_empty_scripts() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .manifest_extra(indoc! {r#"
        [scripts]
        "#})
        .build(&t);

    let output = Scarb::quick_snapbox()
        .args(["--json", "run"])
        .current_dir(&t)
        .stdout_json();

    assert_eq!(output.len(), 0);
}

#[test]
#[cfg_attr(
    not(target_family = "unix"),
    ignore = "This test should write a Rust code, because currently it only assumes Unix."
)]
fn additional_args_passed() {
    let t = TempDir::new().unwrap();
    write_script(
        "hello",
        &formatdoc! {r#"
            #!/usr/bin/env bash
            echo "Hello $@"
        "#},
        &t,
    );

    ProjectBuilder::start()
        .manifest_extra(indoc! {r#"
        [scripts]
        some_script = "scarb-hello"
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .args(["run", "some_script", "--", "beautiful", "world"])
        .current_dir(&t)
        .env("PATH", path_with_temp_dir(&t))
        .assert()
        .success()
        .stdout_eq("Hello beautiful world\n");
}

#[test]
fn pass_exit_code() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .manifest_extra(indoc! {r#"
        [scripts]
        some_script = "exit 21"
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .args(["run", "some_script"])
        .current_dir(&t)
        .assert()
        .failure()
        .code(21);
}

#[test]
fn scripts_shell_uses_current_scarb() {
    let t = TempDir::new().unwrap();

    let script = t.child(format!("scarb{}", env::consts::EXE_SUFFIX));
    script
        .write_str(&formatdoc!(r#"echo "THIS IS A FAKE""#))
        .unwrap();
    make_executable(script.path());

    ProjectBuilder::start()
        .manifest_extra(indoc! {r#"
        [scripts]
        some_script = "scarb --help"
        "#})
        .build(&t);

    let output = Scarb::quick_snapbox()
        .args(["run", "some_script"])
        .current_dir(&t)
        .env("PATH", t.path().to_path_buf().display().to_string())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("The Cairo package manager"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("THIS IS A FAKE"));
}

#[test]
fn uses_package_filter() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .manifest_extra(indoc! {r#"
        [scripts]
        some_script = "echo 'Hello, world!'"
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .args(["run", "-p", "foo", "some_script"])
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("Hello, world!\n");

    Scarb::quick_snapbox()
        .args(["--json", "run", "-p", "bar", "some_script"])
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
        {"type":"error","message":"package `bar` not found in workspace"}
        "#});
}

#[test]
#[cfg_attr(
    not(target_family = "unix"),
    ignore = "This test should write a Rust code, because currently it only assumes Unix."
)]
fn additional_args_not_parsed_as_package_filter() {
    let t = TempDir::new().unwrap();
    write_script(
        "hello",
        &formatdoc! {r#"
            #!/usr/bin/env bash
            echo "Hello $@"
        "#},
        &t,
    );

    ProjectBuilder::start()
        .manifest_extra(indoc! {r#"
        [scripts]
        some_script = "scarb-hello"
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .args(["run", "some_script", "--", "-p", "world"])
        .current_dir(&t)
        .env("PATH", path_with_temp_dir(&t))
        .assert()
        .success()
        .stdout_eq("Hello -p world\n");
}
