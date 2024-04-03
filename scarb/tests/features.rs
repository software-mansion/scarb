use assert_fs::TempDir;
use indoc::indoc;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;


fn build_example_program(t: &TempDir) {
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
            [features]
            x = []
            y = []
            "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn f() -> felt252 { 21 }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { 59 }

            fn main() -> felt252 {
                f()
            }
        "#})
        .build(t);
}

#[test]
fn features_success() {
    let t = TempDir::new().unwrap();
    build_example_program(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--features")
        .arg("x")
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn features_fail_feature_not_enabled() {
    let t = TempDir::new().unwrap();
    build_example_program(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure();
}

#[test]
fn features_unknown_feature() {
    let t = TempDir::new().unwrap();
    build_example_program(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--features")
        .arg("z")
        .current_dir(&t)
        .assert()
        .stdout_matches("error: Unknown features: z\n")
        .failure();
}

// TODO: add tests for default features, --no-default-features and --all-features
