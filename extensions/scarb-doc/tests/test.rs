use scarb_test_support::command::Scarb;

#[test]
fn test_main() {
    let output = Scarb::quick_snapbox()
        .arg("doc")
        .arg("--crate-path")
        .arg("tests/hello_world.cairo")
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
