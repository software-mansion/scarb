use assert_fs::assert::PathAssert;
use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
use indoc::indoc;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn can_run_default_main_function_from_executable() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
                [executable]
            "#})
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 {
                42
            }
        "#})
        .build(&t);

    Scarb::quick_snapbox().arg("build").current_dir(&t).assert();

    let output = Scarb::quick_snapbox()
        .arg("cairo-execute")
        .current_dir(&t)
        .assert()
        .stdout_matches(indoc! {r#"
            [..]Running hello
        "#});

    // assert!(
    //     output.status.success(),
    //     "stdout={}\n stderr={}",
    //     String::from_utf8_lossy(&output.stdout),
    //     String::from_utf8_lossy(&output.stderr),
    // );
    // let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    // assert!(stdout.contains("Running hello"));

    t.child("target/cairo-execute/execution1.zip")
        .assert(predicates::path::exists());
}
