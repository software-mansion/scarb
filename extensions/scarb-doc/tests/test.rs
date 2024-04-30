use assert_fs::TempDir;
use scarb_test_support::{command::Scarb, project_builder::ProjectBuilder};

#[test]
fn test_main() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .version("0.1.0")
        .lib_cairo(include_str!("hello_world.cairo"))
        .build(&t);

    let output = Scarb::quick_snapbox()
        .arg("doc")
        .arg("--crate-path")
        .arg("src/lib.cairo")
        .current_dir(t.path())
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
