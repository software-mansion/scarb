use assert_fs::assert::PathAssert;
use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
use indoc::indoc;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

fn build_executable_project() -> TempDir {
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
    t
}

#[test]
fn can_execute_default_main_function_from_executable() {
    let t = build_executable_project();
    Scarb::quick_snapbox()
        .arg("cairo-execute")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Executing hello
        "#});

    t.child("target/cairo-execute/execution1.zip")
        .assert(predicates::path::exists());
}

#[test]
fn can_execute_prebuilt_executable() {
    let t = build_executable_project();
    Scarb::quick_snapbox().arg("build").current_dir(&t).assert();
    Scarb::quick_snapbox()
        .arg("cairo-execute")
        .arg("--no-build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]Executing hello
        "#});
}

#[test]
fn fails_when_target_missing() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
                [executable]
            "#})
        .lib_cairo(indoc! {r#"
            fn main() -> felt252 {
                42
            }
        "#})
        .build(&t);

    Scarb::quick_snapbox().arg("build").current_dir(&t).assert();

    Scarb::quick_snapbox()
        .arg("cairo-execute")
        .arg("--no-build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..]Executing hello
        error: package has not been compiled, file does not exist: hello.executable.json
        help: run `scarb build` to compile the package
        "#});
}
