use assert_fs::TempDir;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

const FIBONACCI_CODE_WITHOUT_FEATURE: &str = include_str!("code/code_1.cairo");

#[test]
fn can_build_mdbook_with_build_arg() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .edition("2023_01")
        .lib_cairo(FIBONACCI_CODE_WITHOUT_FEATURE)
        .build(&t);

    Scarb::quick_command()
        .arg("doc")
        .arg("--build")
        .current_dir(&t)
        .assert()
        .success();

    assert!(
        t.path().join("target/doc/hello_world/book").exists(),
        "book dir not found"
    );
    assert!(
        t.path().join("target/doc/hello_world/book").is_dir(),
        "book dir not a directory"
    );
    assert!(
        t.path()
            .join("target/doc/hello_world/book/index.html")
            .exists(),
        "index.html not found"
    );
    assert!(
        t.path()
            .join("target/doc/hello_world/book/hello_world.html")
            .exists(),
        "hello_world.html not found"
    );
}
