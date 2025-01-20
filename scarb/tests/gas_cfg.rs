use assert_fs::prelude::*;
use assert_fs::TempDir;
use indoc::indoc;
use scarb_test_support::project_builder::ProjectBuilder;
use scarb_test_support::command::{CommandExt, Scarb};

#[test]
fn gas_enabled_by_default() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
            #[cfg(not(gas: "disabled"))]
            fn f() -> felt252 { 42 }

            #[cfg(gas: "disabled")]
            fn f() -> felt252 { 21 }

            fn main() -> felt252 {
                f()
            }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target/dev/hello.sierra.json")
        .assert(predicates::str::contains(r#""debug_name":"Const<felt252, 42>""#));
}

#[test]
fn gas_disabled_with_config() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(not(gas: "disabled"))]
            fn f() -> felt252 { 42 }

            #[cfg(gas: "disabled")]
            fn f() -> felt252 { 21 }

            fn main() -> felt252 {
                f()
            }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target/dev/hello.sierra.json")
        .assert(predicates::str::contains(r#""debug_name":"Const<felt252, 21>""#));
}

#[test]
fn gas_disabled_in_metadata() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
            [cairo]
            enable-gas = false
        "#})
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<scarb_metadata::Metadata>();

    let unit = &metadata.compilation_units[0];
    assert!(unit.cfg.contains(&scarb_metadata::Cfg::KV("gas".into(), "disabled".into())));
}
