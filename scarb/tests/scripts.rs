use assert_fs::TempDir;
use assert_fs::prelude::*;
use indoc::{formatdoc, indoc};
use itertools::Itertools;
use scarb_test_support::command::{CommandExt, Scarb};
use scarb_test_support::filesystem::{path_with_temp_dir, write_simple_hello_script};
use scarb_test_support::fsx::make_executable;
use scarb_test_support::project_builder::ProjectBuilder;
use scarb_test_support::workspace_builder::WorkspaceBuilder;
use std::collections::BTreeMap;
use std::env;
use std::io::BufRead;

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
            error: missing script `some_other_script` for package: pkg0

            To see a list of scripts, run:
                scarb run
        "#});
}

#[test]
fn run_missing_script_in_workspace() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
        .manifest_extra(indoc! {r#"
        [scripts]
        some_script = "echo 'Hello, world!'"
        "#})
        .build(&t.child("first"));
    ProjectBuilder::start()
        .name("second")
        .build(&t.child("second"));
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .build(&t);
    Scarb::quick_snapbox()
        .args(["run", "-p", "first", "some_other_script"])
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: missing script `some_other_script` for package: first

            To see a list of scripts, run:
                scarb run -p first
        "#});
}

#[test]
fn script_inherits_env_vars() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .manifest_extra(indoc! {r#"
        [scripts]
        some_script = "echo $HELLO"
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .env("HELLO", "Hello, world!")
        .args(["run", "some_script"])
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("Hello, world!\n");
}

#[test]
fn scarb_env_var_cannot_be_overwritten() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .manifest_extra(indoc! {r#"
        [scripts]
        some_script = "echo $SCARB_PROFILE"
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .env("SCARB_PROFILE", "Hello, world!")
        .args(["--release", "run", "some_script"])
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("release\n");
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

    let output: BTreeMap<String, String> = Scarb::quick_snapbox()
        .args(["--json", "run"])
        .current_dir(&t)
        .stdout_json();

    assert_eq!(output["some_script"], "echo 'Hello'");
    assert_eq!(output["some_other_script"], "echo 'world!'");
    assert_eq!(output.len(), 2);

    Scarb::quick_snapbox()
        .arg("run")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            Scripts available via `scarb run`:
            some_other_script     : echo 'world!'
            some_script           : echo 'Hello'

        "#});
}

#[test]
fn list_scripts_in_workspace_with_package_filter() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
        .manifest_extra(indoc! {r#"
            [scripts]
            some_script.workspace = true
            some_other_script.workspace = true
        "#})
        .build(&t.child("first"));
    ProjectBuilder::start()
        .name("second")
        .build(&t.child("second"));
    WorkspaceBuilder::start()
        .manifest_extra(indoc! {r#"
            [workspace.scripts]
            some_script = "echo 'Hello'"
            some_other_script = "echo 'world!'"
        "#})
        .add_member("first")
        .add_member("second")
        .build(&t);

    let output: BTreeMap<String, String> = Scarb::quick_snapbox()
        .args(["--json", "run", "-p", "first"])
        .current_dir(&t)
        .stdout_json();

    assert_eq!(output["some_script"], "echo 'Hello'");
    assert_eq!(output["some_other_script"], "echo 'world!'");
    assert_eq!(output.len(), 2);

    Scarb::quick_snapbox()
        .args(["run", "-p", "first"])
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            Scripts available via `scarb run` for package `first`:
            some_other_script     : echo 'world!'
            some_script           : echo 'Hello'

        "#});
}

#[test]
fn list_scripts_in_workspace() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
        .manifest_extra(indoc! {r#"
            [scripts]
            some_script.workspace = true
            some_other_script.workspace = true
        "#})
        .build(&t.child("first"));
    ProjectBuilder::start()
        .name("second")
        .build(&t.child("second"));
    WorkspaceBuilder::start()
        .manifest_extra(indoc! {r#"
            [workspace.scripts]
            some_script = "echo 'Hello'"
            some_other_script = "echo 'world!'"
        "#})
        .add_member("first")
        .add_member("second")
        .build(&t);

    let output = Scarb::quick_snapbox()
        .args(["--json", "run"])
        .current_dir(&t)
        .output();

    let output = output.expect("Failed to spawn command");
    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = BufRead::split(output.stdout.as_slice(), b'\n')
        .map(|line| line.expect("Failed to read line from stdout"))
        .collect_vec();
    let Some((first, rest)) = output.split_first() else {
        panic!("Expected exactly two lines of output")
    };

    let Some((second, rest)) = rest.split_first() else {
        panic!("Expected exactly two lines of output")
    };

    assert_eq!(rest.len(), 0, "Expected exactly two lines of output");

    match serde_json::de::from_slice::<BTreeMap<String, String>>(first) {
        Ok(output) => {
            assert_eq!(output["some_script"], "echo 'Hello'");
            assert_eq!(output["some_other_script"], "echo 'world!'");
            assert_eq!(output.len(), 2);
        }
        Err(_) => panic!("Cannot deserialize first scripts list"),
    }

    match serde_json::de::from_slice::<BTreeMap<String, String>>(second) {
        Ok(output) => {
            assert_eq!(output, BTreeMap::new());
        }
        Err(_) => panic!("Cannot deserialize second scripts list"),
    }

    Scarb::quick_snapbox()
        .args(["run"])
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            Scripts available via `scarb run` for package `first`:
            some_other_script     : echo 'world!'
            some_script           : echo 'Hello'

            Scripts available via `scarb run` for package `second`:

        "#});
}

#[test]
fn list_empty_scripts() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .manifest_extra(indoc! {r#"
        [scripts]
        "#})
        .build(&t);

    let output: BTreeMap<String, String> = Scarb::quick_snapbox()
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
    write_simple_hello_script("hello", &t);

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
        some_script = "scarb -h"
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

    let first = t.child("first");
    ProjectBuilder::start()
        .name("first")
        .manifest_extra(indoc! {r#"
        [scripts]
        some_script = "echo 'Hello first package!'"
        "#})
        .build(&first);
    let second = t.child("second");
    ProjectBuilder::start()
        .name("second")
        .manifest_extra(indoc! {r#"
        [scripts]
        some_script = "echo 'Hello second package!'"
        "#})
        .build(&second);
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .build(&t);

    Scarb::quick_snapbox()
        .args(["run", "-p", "first", "some_script"])
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("Hello first package!\n");

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
fn package_filter_from_env() {
    let t = TempDir::new().unwrap();

    let first = t.child("first");
    ProjectBuilder::start()
        .name("first")
        .manifest_extra(indoc! {r#"
        [scripts]
        some_script = "echo 'Hello first package!'"
        "#})
        .build(&first);
    let second = t.child("second");
    ProjectBuilder::start()
        .name("second")
        .manifest_extra(indoc! {r#"
        [scripts]
        some_script = "echo 'Broke!'"
        "#})
        .build(&second);
    let second = t.child("third");
    ProjectBuilder::start()
        .name("third")
        .manifest_extra(indoc! {r#"
        [scripts]
        some_script = "echo 'Hello third package!'"
        "#})
        .build(&second);
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .add_member("third")
        .build(&t);

    let output = Scarb::quick_snapbox()
        .env("SCARB_PACKAGES_FILTER", "first,third")
        .args(["run", "some_script"])
        .current_dir(&t)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "Command failed! {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.eq("Hello first package!\nHello third package!\n")
            || stdout.eq("Hello third package!\nHello first package!\n")
    )
}

#[test]
#[cfg_attr(
    not(target_family = "unix"),
    ignore = "This test should write a Rust code, because currently it only assumes Unix."
)]
fn additional_args_not_parsed_as_package_filter() {
    let t = TempDir::new().unwrap();
    write_simple_hello_script("hello", &t);

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

#[test]
fn run_missing_script_in_workspace_root() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
        .manifest_extra(indoc! {r#"
        [scripts]
        some_script.workspace = true
        "#})
        .build(&t.child("first"));
    ProjectBuilder::start()
        .name("second")
        .build(&t.child("second"));
    WorkspaceBuilder::start()
        .manifest_extra(indoc! {r#"
        [workspace.scripts]
        some_script = "echo 'Hello, world!'"
        "#})
        .add_member("first")
        .add_member("second")
        .build(&t);
    Scarb::quick_snapbox()
        .args(["run", "--workspace-root", "some_other_script"])
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: missing script `some_other_script` for workspace root

            To see a list of scripts, run:
                scarb run --workspace-root
        "#});
}

#[test]
fn list_scripts_in_workspace_root() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
        .manifest_extra(indoc! {r#"
            [scripts]
            some_package_script = "echo 'Hello'"
        "#})
        .build(&t.child("first"));
    ProjectBuilder::start()
        .name("second")
        .build(&t.child("second"));
    WorkspaceBuilder::start()
        .manifest_extra(indoc! {r#"
            [workspace.scripts]
            some_script = "echo 'Hello'"
            some_other_script = "echo 'world!'"
        "#})
        .add_member("first")
        .add_member("second")
        .build(&t);

    Scarb::quick_snapbox()
        .args(["run", "--workspace-root"])
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            Scripts available via `scarb run` for workspace root:
            some_other_script     : echo 'world!'
            some_script           : echo 'Hello'

        "#});

    let output: BTreeMap<String, String> = Scarb::quick_snapbox()
        .args(["--json", "run", "--workspace-root"])
        .current_dir(&t)
        .stdout_json();

    assert_eq!(output["some_script"], "echo 'Hello'");
    assert_eq!(output["some_other_script"], "echo 'world!'");
    assert_eq!(output.len(), 2);
}

#[test]
fn run_workspace_root_script() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
        .manifest_extra(indoc! {r#"
            [scripts]
            pwd.workspace = true
        "#})
        .build(&t.child("first"));
    ProjectBuilder::start()
        .name("second")
        .build(&t.child("second"));
    WorkspaceBuilder::start()
        .manifest_extra(indoc! {r#"
            [workspace.scripts]
            pwd = "pwd"
        "#})
        .add_member("first")
        .add_member("second")
        .build(&t);
    let output = Scarb::quick_snapbox()
        .args(["run", "-p", "first", "pwd"])
        .current_dir(&t)
        .output()
        .expect("failed to spawn command");
    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let pkg_pwd = String::from_utf8_lossy(&output.stdout).to_string();
    let output = Scarb::quick_snapbox()
        .args(["run", "--workspace-root", "pwd"])
        .current_dir(&t)
        .output()
        .expect("failed to spawn command");
    assert!(
        output.status.success(),
        "Command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let ws_pwd = String::from_utf8_lossy(&output.stdout).to_string();
    assert_ne!(pkg_pwd, ws_pwd);
}
