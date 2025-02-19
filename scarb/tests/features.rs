use assert_fs::assert::PathAssert;
use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
use indoc::indoc;

use scarb_metadata::{Cfg, Metadata};
use scarb_test_support::command::{CommandExt, Scarb};
use scarb_test_support::project_builder::ProjectBuilder;
use scarb_test_support::workspace_builder::WorkspaceBuilder;

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

fn build_with_default_features(t: &TempDir) {
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
            [features]
            default = ["x", "y"]
            x = []
            y = []
            "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn g() -> felt252 { 21 }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { g() }

            fn main() -> felt252 {
                f()
            }
        "#})
        .build(t);
}

fn build_with_all_features_required(t: &TempDir) {
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
            [features]
            w = []
            x = []
            y = []
            z = []
            "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn f() -> felt252 { 22 }

            #[cfg(feature: 'y')]
            fn g() -> felt252 { f() }

            #[cfg(feature: 'z')]
            fn h() -> felt252 { g() }

            #[cfg(feature: 'w')]
            fn i() -> felt252 { h() }

            fn main() -> felt252 {
                i()
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
        .stdout_matches(indoc! {r#"
            [..] Compiling hello v1.0.0 ([..])
            error: The name `f` is defined multiple times.
             --> [..]/src/lib.cairo[..]
            fn f() -> felt252 { 59 }
               ^
            
            error: could not compile `hello` due to previous error
        "#})
        .failure();
}

#[test]
fn features_fail_no_feature_enabled() {
    let t = TempDir::new().unwrap();
    build_example_program(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .stdout_matches(indoc! {r#"
            [..] Compiling hello v1.0.0 ([..])
            error: Function not found.
             --> [..]/src/lib.cairo[..]
                f()
                ^

            error: could not compile `hello` due to previous error
        "#})
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
        .stdout_matches(indoc! {r#"
            error: none of the selected packages contains `z` feature
            note: to use features, you need to define [features] section in Scarb.toml
        "#})
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
        .stdout_matches(indoc! {r#"
            error: none of the selected packages contains `x` feature
            note: to use features, you need to define [features] section in Scarb.toml
        "#})
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
                TOML parse error at line 9, column 1
                  |
                9 | 8x = []
                  | ^^
                the name `8x` cannot be used as a package name, names cannot start with a digit
        "#})
        .failure();
}

#[test]
fn features_with_default_features() {
    let t = TempDir::new().unwrap();
    build_with_default_features(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target/dev/hello.sierra.json")
        .assert(predicates::str::contains(r#""debug_name":"hello::main""#));
}

#[test]
fn features_no_default_features() {
    let t = TempDir::new().unwrap();
    build_with_default_features(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--no-default-features")
        .current_dir(&t)
        .assert()
        .stdout_matches(indoc! {r#"
            [..] Compiling hello v1.0.0 ([..])
            error: Function not found.
             --> [..]/src/lib.cairo[..]
                f()
                ^

            error: could not compile `hello` due to previous error
        "#})
        .failure();
}

#[test]
fn features_all_features() {
    let t = TempDir::new().unwrap();
    build_with_all_features_required(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--all-features")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target/dev/hello.sierra.json")
        .assert(predicates::str::contains(r#""debug_name":"hello::main""#));
}

#[test]
fn features_all_features_failing() {
    let t = TempDir::new().unwrap();
    build_with_all_features_required(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .stdout_matches(indoc! {r#"
            [..] Compiling hello v1.0.0 ([..])
            error: Function not found.
             --> [..]/src/lib.cairo[..]
                i()
                ^

            error: could not compile `hello` due to previous error
        "#})
        .failure();
}

#[test]
fn features_no_default_and_all_failing() {
    let t = TempDir::new().unwrap();
    build_with_default_features(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--no-default-features")
        .arg("--all-features")
        .current_dir(&t)
        .assert()
        .stderr_matches(indoc! {r#"
            error: the argument '--no-default-features' cannot be used with '--all-features'

            Usage: scarb[..] build --no-default-features

            For more information, try '--help'.
        "#})
        .failure();
}

#[test]
fn features_metadata_feature_in_compilation_units() {
    let t = TempDir::new().unwrap();
    build_example_program(&t);
    let output = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--features")
        .arg("x")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert!(!output.compilation_units.is_empty());
    let unit = &output.compilation_units[0];
    assert!(unit.package.repr.starts_with("hello "));
    assert_eq!(unit.target.name, "hello");
    assert!(!unit.components.is_empty());
    assert!(unit
        .cfg
        .contains(&Cfg::KV("target".into(), unit.target.kind.clone())));
    assert!(unit.components.len() >= 2);
    let main_component_cfg = unit.components[1].cfg.clone();
    assert!(
        main_component_cfg.is_some_and(|cfg| cfg.contains(&Cfg::KV("feature".into(), "x".into())))
    );
}

#[test]
fn features_in_workspace_success() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
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
        .build(&t.child("first"));
    ProjectBuilder::start()
        .name("second")
        .lib_cairo(indoc! {r#"
            fn main() -> felt252 {
                12
            }
        "#})
        .build(&t.child("second"));
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("check")
        .arg("--package")
        .arg("first")
        .arg("--features")
        .arg("x")
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn features_in_workspace_validated() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
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
        .build(&t.child("first"));
    ProjectBuilder::start()
        .name("second")
        .lib_cairo(indoc! {r#"
            fn main() -> felt252 {
                12
            }
        "#})
        .build(&t.child("second"));
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("check")
        .arg("--package")
        .arg("second")
        .arg("--features")
        .arg("x")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: none of the selected packages contains `x` feature
            note: to use features, you need to define [features] section in Scarb.toml
        "#});
}
