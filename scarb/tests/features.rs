use assert_fs::assert::PathAssert;
use assert_fs::fixture::PathChild;
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

fn build_missing_manifest_example_program(t: &TempDir) {
    ProjectBuilder::start()
        .name("hello")
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

fn build_incorrect_manifest_feature_example_program(t: &TempDir) {
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
            [features]
            8x = []
            y = []
            "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: '8x')]
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

    t.child("target/dev/hello.sierra.json")
        .assert(predicates::str::contains(r#""debug_name":"hello::f""#));

    build_example_program(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--features")
        .arg("y")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target/dev/hello.sierra.json")
        .assert(predicates::str::contains(r#""debug_name":"hello::f""#));
}

#[test]
fn features_fail_both_features_enabled() {
    let t = TempDir::new().unwrap();
    build_example_program(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--features")
        .arg("x,y")
        .current_dir(&t)
        .assert()
        .failure();
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
        .stdout_matches("error: unknown features: z\n")
        .failure();
}

#[test]
fn features_fail_missing_manifest() {
    let t = TempDir::new().unwrap();
    build_missing_manifest_example_program(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--features")
        .arg("x")
        .current_dir(&t)
        .assert()
        .stdout_matches("error: no features in manifest\n")
        .failure();
}

#[test]
fn features_fail_incorrect_manifest() {
    let t = TempDir::new().unwrap();
    build_incorrect_manifest_feature_example_program(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--features")
        .arg("x")
        .current_dir(&t)
        .assert()
        .stdout_matches(indoc! {r#"
            error: failed to parse manifest at: [..]/Scarb.toml

            Caused by:
                TOML parse error at line 8, column 1
                  |
                8 | 8x = []
                  | ^^
                the name `8x` cannot be used as a package name, names cannot start with a digit
        "#})
        .failure();
}

// TODO: add tests for default features, --no-default-features and --all-features
