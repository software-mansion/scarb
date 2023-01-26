use assert_fs::prelude::*;
use assert_fs::TempDir;
use indoc::indoc;
use std::fs;

use crate::support::command::Scarb;

const SIMPLE_ORIGINAL: &str = r"fn main()    ->    felt      {      42      }";
const SIMPLE_FORMATTED: &str = indoc! {r#"
    fn main() -> felt {
        42
    }
    "#
};

fn build_temp_dir(data: &str) -> TempDir {
    let t = assert_fs::TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();
    t.child("src/lib.cairo").write_str(data).unwrap();

    t
}

#[test]
fn simple_check_invalid() {
    let t = build_temp_dir(SIMPLE_ORIGINAL);
    Scarb::quick_snapbox()
        .arg("fmt")
        .arg("--check")
        .arg("--no-color")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {"\
            Diff in [..]/src/lib.cairo:
             --- original
            +++ modified
            @@ -1 +1,3 @@
            -fn main()    ->    felt      {      42      }
            / No newline at end of file
            +fn main() -> felt {
            +    42
            +}

            "});
    let content = fs::read_to_string(t.child("src/lib.cairo")).unwrap();
    assert_eq!(content, SIMPLE_ORIGINAL);
}

#[test]
fn simple_check_valid() {
    let t = build_temp_dir(SIMPLE_FORMATTED);
    Scarb::quick_snapbox()
        .arg("fmt")
        .arg("--check")
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn simple_format() {
    let t = build_temp_dir(SIMPLE_ORIGINAL);
    Scarb::quick_snapbox()
        .arg("fmt")
        .current_dir(&t)
        .assert()
        .success();

    assert!(t.child("src/lib.cairo").is_file());
    let content = fs::read_to_string(t.child("src/lib.cairo")).unwrap();
    assert_eq!(content, SIMPLE_FORMATTED);
}
