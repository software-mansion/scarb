use std::fs;

use assert_fs::TempDir;
use scarb_test_support::command::Scarb;

#[test]
fn test_main() {
    let tempdir = TempDir::new().unwrap();
    fs::copy(
        "tests/hello_world.cairo",
        tempdir.path().join("hello_world.cairo"),
    )
    .unwrap();

    let output = Scarb::quick_snapbox()
        .arg("doc")
        .arg("--crate-path")
        .arg("hello_world.cairo")
        .current_dir(tempdir.path())
        .assert()
        .success();
    let stdout = std::str::from_utf8(&output.get_output().stdout).unwrap();

    let expected_items = [
        "main",
        "FOO",
        "fib",
        "Pair",
        "Color",
        "Shape",
        "Circle",
        "CircleShape",
        "tests",
        "fib_function",
        "it_works",
    ];
    for item in expected_items.iter() {
        assert!(stdout.contains(item));
    }
}
