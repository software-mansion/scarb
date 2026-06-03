use assert_fs::TempDir;
use assert_fs::prelude::*;
use indoc::indoc;
use serde_json::Value;

use scarb_test_support::command::Scarb;
use scarb_test_support::workspace_builder::WorkspaceBuilder;

#[test]
fn warn_on_unknown_top_level_section() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(indoc! {r#"
            [package]
            name = "hello"
            version = "0.1.0"
            edition = "2024_07"

            [unknown-section]
            bar = "baz"
        "#})
        .unwrap();
    t.child("src/lib.cairo").write_str("").unwrap();

    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("warn: unknown manifest field `unknown-section` ([..]/Scarb.toml:6:2)\n");
}

#[test]
fn warn_on_unknown_package_field() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(indoc! {r#"
            [package]
            name = "hello"
            version = "0.1.0"
            edition = "2024_07"
            typo_field = "oops"
        "#})
        .unwrap();
    t.child("src/lib.cairo").write_str("").unwrap();

    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("warn: unknown manifest field `package.typo_field` ([..]/Scarb.toml:5:1)\n");
}

#[test]
fn no_warn_for_known_tool_and_profile_sections() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(indoc! {r#"
            [package]
            name = "hello"
            version = "0.1.0"
            edition = "2024_07"

            [tool.starknet-foundry]
            version = "0.1.0"

            [profile.dev]
            [profile.dev.cairo]
            enable-gas = false
        "#})
        .unwrap();
    t.child("src/lib.cairo").write_str("").unwrap();

    // No unknown-field warnings expected — output should be empty.
    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("");
}

#[test]
fn warn_on_unknown_field_in_workspace_member() {
    let t = TempDir::new().unwrap().child("ws");
    let member = t.child("member");

    member
        .child("Scarb.toml")
        .write_str(indoc! {r#"
            [package]
            name = "member"
            version = "0.1.0"
            edition = "2024_07"

            [mystery-key]
            x = 1
        "#})
        .unwrap();
    member.child("src/lib.cairo").write_str("").unwrap();

    WorkspaceBuilder::start().add_member("member").build(&t);

    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("warn: unknown manifest field `mystery-key` ([..]/Scarb.toml:6:2)\n");
}

#[test]
fn warn_once_when_root_manifest_is_workspace_and_package() {
    let t = TempDir::new().unwrap();
    let member = t.child("member");

    t.child("Scarb.toml")
        .write_str(indoc! {r#"
            [package]
            name = "root"
            version = "0.1.0"
            edition = "2024_07"
            typo_field = "oops"

            [workspace]
            members = ["member"]
        "#})
        .unwrap();
    t.child("src/lib.cairo").write_str("").unwrap();

    member
        .child("Scarb.toml")
        .write_str(indoc! {r#"
            [package]
            name = "member"
            version = "0.1.0"
            edition = "2024_07"

            [mystery-key]
            x = 1
        "#})
        .unwrap();
    member.child("src/lib.cairo").write_str("").unwrap();

    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {"\
            warn: unknown manifest field `package.typo_field` ([..]/Scarb.toml:5:1)
            warn: unknown manifest field `mystery-key` ([..]/member/Scarb.toml:6:2)
        "});
}

#[test]
fn warn_on_unknown_field_in_nullable_workspace_package_section() {
    let t = TempDir::new().unwrap().child("ws");
    let member = t.child("member");

    t.child("Scarb.toml")
        .write_str(indoc! {r#"
            [workspace]
            members = ["member"]

            [workspace.package]
            version = "0.1.0"
            edition = "2024_07"
            typo_field = "oops"
        "#})
        .unwrap();

    member
        .child("Scarb.toml")
        .write_str(indoc! {r#"
            [package]
            name = "member"
            version.workspace = true
            edition.workspace = true
        "#})
        .unwrap();
    member.child("src/lib.cairo").write_str("").unwrap();

    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(
            "warn: unknown manifest field `workspace.package.typo_field` ([..]/Scarb.toml:7:1)\n",
        );
}

#[test]
fn unknown_field_warning_span_points_to_correct_line() {
    let t = TempDir::new().unwrap();
    // `[bad-field]` starts on line 6 (1-based) in the de-indented file.
    t.child("Scarb.toml")
        .write_str(indoc! {r#"
            [package]
            name = "hello"
            version = "0.1.0"
            edition = "2024_07"

            [bad-field]
            x = 1
        "#})
        .unwrap();
    t.child("src/lib.cairo").write_str("").unwrap();

    let output = Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();

    assert!(
        stdout.contains("unknown manifest field `bad-field`"),
        "expected unknown-field warning in output:\n{stdout}"
    );
    // The span must point to line 6 where [bad-field] appears.
    assert!(
        stdout.contains(":6:"),
        "expected line 6 in span, got:\n{stdout}"
    );
}

/// Finds the first NDJSON line with `kind == "manifest_diagnostic"`.
fn find_diagnostic(stdout: &str) -> Value {
    stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|v| v["kind"] == "manifest_diagnostic")
        .expect("no manifest_diagnostic line found in JSON output")
}

#[test]
fn json_mode_emits_manifest_diagnostic_for_unknown_top_level_section() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(indoc! {r#"
            [package]
            name = "hello"
            version = "0.1.0"
            edition = "2024_07"

            [unknown-section]
            bar = "baz"
        "#})
        .unwrap();
    t.child("src/lib.cairo").write_str("").unwrap();

    let output = Scarb::quick_command()
        .arg("--json")
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();
    let diag = find_diagnostic(&stdout);

    assert!(
        diag["message"]
            .as_str()
            .unwrap()
            .contains("unknown manifest field `unknown-section`"),
        "unexpected message: {}",
        diag["message"]
    );
    assert_eq!(diag["error_code"].as_str().unwrap(), "SE0002");
    assert!(diag["file"].is_string(), "expected file field");
    assert!(diag["span"].is_object(), "expected span field");
    assert!(
        diag.get("severity").is_none(),
        "did not expect severity field"
    );
}

#[test]
fn json_mode_emits_manifest_diagnostic_for_unknown_package_field() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(indoc! {r#"
            [package]
            name = "hello"
            version = "0.1.0"
            edition = "2024_07"
            typo_field = "oops"
        "#})
        .unwrap();
    t.child("src/lib.cairo").write_str("").unwrap();

    let output = Scarb::quick_command()
        .arg("--json")
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();
    let diag = find_diagnostic(&stdout);

    assert!(
        diag["message"]
            .as_str()
            .unwrap()
            .contains("unknown manifest field `package.typo_field`"),
        "unexpected message: {}",
        diag["message"]
    );
    assert_eq!(diag["error_code"].as_str().unwrap(), "SE0002");
    assert!(diag["file"].is_string(), "expected file field");
    assert!(diag["span"].is_object(), "expected span field");
}

#[test]
fn json_mode_diagnostic_span_has_correct_byte_offset() {
    let t = TempDir::new().unwrap();
    // `[bad-field]` is on line 6 of the written file.
    t.child("Scarb.toml")
        .write_str(indoc! {r#"
            [package]
            name = "hello"
            version = "0.1.0"
            edition = "2024_07"

            [bad-field]
            x = 1
        "#})
        .unwrap();
    t.child("src/lib.cairo").write_str("").unwrap();

    let output = Scarb::quick_command()
        .arg("--json")
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();
    let diag = find_diagnostic(&stdout);

    let span = &diag["span"];
    assert!(span.is_object(), "expected span object");
    // `start` must be a non-zero byte offset (the field is not at the very beginning).
    let start = span["start"].as_u64().expect("span.start must be a number");
    assert!(start > 0, "span.start should be > 0, got {start}");
}
