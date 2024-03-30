use assert_fs::TempDir;
use indoc::indoc;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn feature_no_features_failing() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(
            r#"
            [features]
            x = []
            y = []
            "#,
        )
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn f() -> felt252 { 21 }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { 59 }

            fn main() -> felt252 {
                f()
            }
        "#})
        .build(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure();
}

#[test]
fn feature_with_feature() {
    let t = TempDir::new().unwrap();
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
        .build(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--features")
        .arg("x")
        .current_dir(&t)
        .assert()
        .success();
}

// TODO: add tests for default features, --no-default-features and --all-features
