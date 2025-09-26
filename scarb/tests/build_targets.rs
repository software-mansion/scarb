use assert_fs::TempDir;
use assert_fs::prelude::*;
use cairo_lang_sierra::program::VersionedProgram;
use cairo_lang_starknet_classes::contract_class::ContractClass;
use indoc::{formatdoc, indoc};
use itertools::Itertools;
use predicates::prelude::*;
use scarb_metadata::Metadata;
use scarb_test_support::command::{CommandExt, Scarb};
use scarb_test_support::contracts::{BALANCE_CONTRACT, FORTY_TWO_CONTRACT, HELLO_CONTRACT};
use scarb_test_support::fsx;
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
use scarb_test_support::workspace_builder::WorkspaceBuilder;
use serde_json::json;
use std::path::PathBuf;
use test_case::test_case;

#[test]
fn compile_with_duplicate_targets_1() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            edition = "2023_01"

            [[target.example]]

            [[target.example]]
            "#,
        )
        .unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: failed to parse manifest at: [..]/Scarb.toml

        Caused by:
            manifest contains duplicate target definitions `example`, consider explicitly naming targets with the `name` field
        "#});
}

#[test]
fn compile_with_duplicate_targets_2() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            edition = "2023_01"

            [[target.example]]
            name = "x"

            [[target.example]]
            name = "x"
            "#,
        )
        .unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: failed to parse manifest at: [..]/Scarb.toml

        Caused by:
            manifest contains duplicate target definitions `example (x)`, use different target names to resolve the conflict
        "#});
}

#[test]
fn compile_with_custom_lib_target() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            edition = "2023_01"

            [lib]
            name = "not_hello"
            sierra = false
            casm = true
            sierra-text = true
            "#,
        )
        .unwrap();
    t.child("src/lib.cairo")
        .write_str(r#"fn f() -> felt252 { 42 }"#)
        .unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        [..]  Finished `dev` profile target(s) in [..]
        "#});

    t.child("target/dev/not_hello.casm")
        .assert(predicates::str::is_empty().not());
    t.child("target/dev/not_hello.sierra")
        .assert(predicates::str::is_empty().not());
    t.child("target/dev/not_hello.sierra.json")
        .assert(predicates::path::exists().not());
    t.child("target/dev/hello.sierra.json")
        .assert(predicates::path::exists().not());
    t.child("target/dev/hello.casm")
        .assert(predicates::path::exists().not());
    t.child("target/dev/hello.sierra")
        .assert(predicates::path::exists().not());
}

#[test]
fn compile_with_named_default_lib_target() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            edition = "2023_01"

            [lib]
            name = "not_hello"
            "#,
        )
        .unwrap();
    t.child("src/lib.cairo")
        .write_str(r#"fn f() -> felt252 { 42 }"#)
        .unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        [..]  Finished `dev` profile target(s) in [..]
        "#});

    t.child("target/dev/not_hello.sierra.json")
        .assert(predicates::str::is_empty().not());
    t.child("target/dev/not_hello.sierra")
        .assert(predicates::path::exists().not());
    t.child("target/dev/not_hello.casm")
        .assert(predicates::path::exists().not());
    t.child("target/dev/hello.sierra.json")
        .assert(predicates::path::exists().not());
    t.child("target/dev/hello.casm")
        .assert(predicates::path::exists().not());
    t.child("target/dev/hello.sierra")
        .assert(predicates::path::exists().not());
}

#[test]
fn compile_with_lib_target_in_target_array() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            edition = "2023_01"

            [[target.lib]]
            name = "not_hello"
            sierra = true
            "#,
        )
        .unwrap();
    t.child("src/lib.cairo")
        .write_str(r#"fn f() -> felt252 { 42 }"#)
        .unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        [..]  Finished `dev` profile target(s) in [..]
        "#});

    t.child("target/dev/not_hello.sierra.json")
        .assert(predicates::str::is_empty().not());
}

#[test]
fn compile_dep_not_a_lib() {
    let t = TempDir::new().unwrap();

    let dep = t.child("dep");
    ProjectBuilder::start()
        .name("dep")
        .version("1.0.0")
        .manifest_extra("[[target.starknet-contract]]")
        .lib_cairo("fn forty_two() -> felt252 { 42 }")
        .build(&dep);

    let hello = t.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep", &dep)
        .lib_cairo("fn hellp() -> felt252 { dep::forty_two() }")
        .build(&hello);

    Scarb::quick_snapbox()
        .arg("check")
        .current_dir(&hello)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            warn: hello v1.0.0 ([..]) ignoring invalid dependency `dep` which is missing a lib or cairo-plugin target
                Checking hello v1.0.0 ([..])
            error[E0006]: Identifier not found.
             --> [..]/lib.cairo:1:25
            fn hellp() -> felt252 { dep::forty_two() }
                                    ^^^

            error: could not check `hello` due to previous error
        "#});
}

#[test]
fn target_with_source_path() {
    let t = TempDir::new().unwrap();
    t.child("tests/x.cairo")
        .write_str(r#"fn f() -> felt252 { 42 }"#)
        .unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"

            [[target.test]]
            source-path = "tests/x.cairo"

            "#,
        )
        .unwrap();

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn target_source_path_disallowed() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"

            [lib]
            source-path = "src/example.cairo"
            "#,
        )
        .unwrap();

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: failed to parse manifest at: [..]/Scarb.toml

            Caused by:
                `lib` target cannot specify custom `source-path`
        "#});
}

#[test]
fn test_target_skipped_without_flag() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    assert_eq!(t.child("target").files(), vec!["CACHEDIR.TAG", "dev"]);
    assert_eq!(
        t.child("target/dev").files(),
        vec![".fingerprint", "hello.sierra.json", "incremental"]
    );
}

#[test]
fn compile_test_target() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .dep_cairo_test()
        .lib_cairo(r#"fn f() -> felt252 { 42 }"#)
        .build(&t);
    t.child("tests").create_dir_all().unwrap();
    t.child("tests/test1.cairo")
        .write_str(indoc! {r#"
        #[cfg(test)]
        mod tests {
            use hello::f;
            #[test]
            fn it_works() {
                assert(f() == 42, 'it works!');
            }
        }
         "#})
        .unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .arg("--test")
        .current_dir(&t)
        .assert()
        .success();

    assert_eq!(t.child("target").files(), vec!["CACHEDIR.TAG", "dev"]);
    assert_eq!(
        t.child("target/dev").files(),
        vec![
            ".fingerprint",
            "hello_integrationtest.test.json",
            "hello_integrationtest.test.sierra.json",
            "hello_unittest.test.json",
            "hello_unittest.test.sierra.json",
            "incremental"
        ]
    );

    t.child("target/dev/hello_integrationtest.test.json")
        .assert_is_json::<serde_json::Value>();
    t.child("target/dev/hello_integrationtest.test.sierra.json")
        .assert_is_json::<VersionedProgram>();
    let content = t
        .child("target/dev/hello_integrationtest.test.json")
        .read_to_string();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    let tests = json.get("named_tests").unwrap().as_array().unwrap();
    assert_eq!(tests.len(), 1);

    t.child("target/dev/hello_unittest.test.json")
        .assert_is_json::<serde_json::Value>();
    t.child("target/dev/hello_unittest.test.sierra.json")
        .assert_is_json::<serde_json::Value>();
    let content = t
        .child("target/dev/hello_unittest.test.json")
        .read_to_string();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    let tests = json.get("named_tests").unwrap().as_array().unwrap();
    assert_eq!(tests.len(), 0);
}

#[test]
fn integration_tests_do_not_enable_cfg_in_main_package() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .dep_cairo_test()
        .lib_cairo(indoc! {r#"
            #[cfg(test)]
            fn f() -> felt252 { 42 }
        "#})
        .build(&t);
    t.child("tests").create_dir_all().unwrap();
    t.child("tests/test1.cairo")
        .write_str(indoc! {r#"
        #[cfg(test)]
        mod tests {
            use hello::f;
            #[test]
            fn it_works() {
                assert(f() == 42, 'it works!');
            }
        }
         "#})
        .unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .arg("--test")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            [..]Compiling test(hello_unittest) hello v1.0.0 ([..]Scarb.toml)
            [..]Compiling test(hello_integrationtest) hello_integrationtest v1.0.0 ([..]Scarb.toml)
            error[E0006]: Identifier not found.
             --> [..]test1.cairo:3:16
                use hello::f;
                           ^

            error: Type annotations needed. Failed to infer ?0.
             --> [..]test1.cairo:6:16
                    assert(f() == 42, 'it works!');
                           ^^^^^^^^^

            error: could not compile `hello_integrationtest` due to previous error
        "#});
}

#[test]
fn integration_tests_cannot_use_itself_by_target_name() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .dep_cairo_test()
        .lib_cairo(indoc! {r#"
            fn hello_world() -> felt252 { 42 }
        "#})
        .build(&t);
    t.child("tests").create_dir_all().unwrap();
    t.child("tests/test1.cairo")
        .write_str(indoc! {r#"
        pub fn hello() -> felt252 { 12 }
        pub fn beautiful() -> felt252 { 34 }
        pub fn world() -> felt252 { 56 }

        mod tests {
            use hello_integrationtest::test1::world;
            use hello_tests::test1::beautiful;
            use crate::test1::hello;

            #[test]
            fn test_1() {
                assert(world() == 12, '');
            }
        }
        "#})
        .unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .arg("--test")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            [..]Compiling test(hello_unittest) hello v1.0.0 ([..]Scarb.toml)
            [..]Compiling test(hello_integrationtest) hello_integrationtest v1.0.0 ([..]Scarb.toml)
            error[E0006]: Identifier not found.
             --> [..]test1.cairo:6:9
                use hello_integrationtest::test1::world;
                    ^^^^^^^^^^^^^^^^^^^^^

            error[E0006]: Identifier not found.
             --> [..]test1.cairo:7:9
                use hello_tests::test1::beautiful;
                    ^^^^^^^^^^^

            error: Type annotations needed. Failed to infer ?0.
             --> [..]test1.cairo:12:16
                    assert(world() == 12, '');
                           ^^^^^^^^^^^^^

            error: could not compile `hello_integrationtest` due to previous error
        "#});
}

#[test]
fn features_enabled_in_integration_tests() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .dep_cairo_test()
        .manifest_extra(indoc! {r#"
            [features]
            x = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn f() -> felt252 { 42 }

            fn main() -> felt252 {
                0
            }
        "#})
        .build(&t);

    t.child("tests/test1.cairo")
        .write_str(indoc! {r#"
            #[cfg(test)]
            mod tests {
                use hello::f;

                #[test]
                fn test_feature_function() {
                    assert(f() == 42, 'it works!');
                }
            }
        "#})
        .unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .arg("--test")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            [..] Compiling test(hello_unittest) hello v1.0.0 ([..]Scarb.toml)
            [..] Compiling test(hello_integrationtest) hello_integrationtest v1.0.0 ([..])
            error[E0006]: Identifier not found.
             --> [..]test1.cairo:3:16
                use hello::f;
                           ^

            error: Type annotations needed. Failed to infer ?0.
             --> [..]test1.cairo:7:16
                    assert(f() == 42, 'it works!');
                           ^^^^^^^^^

            error: could not compile `hello_integrationtest` due to previous error
        "#});

    Scarb::quick_snapbox()
        .arg("build")
        .arg("--test")
        .arg("--features")
        .arg("x")
        .current_dir(&t)
        .assert()
        .success();

    assert_eq!(
        t.child("target/dev").files(),
        vec![
            ".fingerprint",
            "hello_integrationtest.test.json",
            "hello_integrationtest.test.sierra.json",
            "hello_unittest.test.json",
            "hello_unittest.test.sierra.json",
            "incremental",
        ]
    );

    t.child("target/dev/hello_integrationtest.test.json")
        .assert_is_json::<serde_json::Value>();
    t.child("target/dev/hello_integrationtest.test.sierra.json")
        .assert_is_json::<VersionedProgram>();
    let content = t
        .child("target/dev/hello_integrationtest.test.json")
        .read_to_string();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    let tests = json.get("named_tests").unwrap().as_array().unwrap();
    assert_eq!(tests.len(), 1);
}

#[test]
fn detect_single_file_test_targets() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);
    t.child("tests/test2").create_dir_all().unwrap();
    t.child("tests/test1.cairo").write_str("").unwrap();
    t.child("tests/test2.cairo").write_str("").unwrap();

    let metadata = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version=1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let test_cu: Vec<(PathBuf, String)> = metadata
        .compilation_units
        .iter()
        .filter(|cu| cu.target.kind == "test")
        .map(|cu| {
            (
                cu.target.source_path.clone().into_std_path_buf(),
                cu.target
                    .params
                    .as_object()
                    .unwrap()
                    .get("test-type")
                    .unwrap()
                    .to_string(),
            )
        })
        .sorted()
        .collect();

    assert_eq!(
        test_cu,
        vec![
            (
                fsx::canonicalize(t.child("src/lib.cairo")).unwrap(),
                r#""unit""#.into()
            ),
            (
                fsx::canonicalize(t.child("tests/test1.cairo")).unwrap(),
                r#""integration""#.into()
            ),
            (
                fsx::canonicalize(t.child("tests/test2.cairo")).unwrap(),
                r#""integration""#.into()
            ),
        ]
    );
}

#[test]
fn autodetect_test_target_non_cairo_files() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);
    t.child("tests/test1.cairo").write_str("").unwrap();
    t.child("tests/Scarb.toml").write_str("").unwrap();
    let metadata = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version=1")
        .current_dir(&t)
        .stdout_json::<Metadata>();
    let test_cu: Vec<(PathBuf, String)> = metadata
        .compilation_units
        .iter()
        .filter(|cu| cu.target.kind == "test")
        .map(|cu| {
            (
                cu.target.source_path.clone().into_std_path_buf(),
                cu.target
                    .params
                    .as_object()
                    .unwrap()
                    .get("test-type")
                    .unwrap()
                    .to_string(),
            )
        })
        .sorted()
        .collect();

    assert_eq!(
        test_cu,
        vec![
            (
                fsx::canonicalize(t.child("src/lib.cairo")).unwrap(),
                r#""unit""#.into()
            ),
            (
                fsx::canonicalize(t.child("tests/test1.cairo")).unwrap(),
                r#""integration""#.into()
            ),
        ]
    );
}

#[test]
fn detect_lib_test_target() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);
    t.child("tests/test2").create_dir_all().unwrap();
    t.child("tests/test1.cairo").write_str("").unwrap();
    t.child("tests/test2.cairo").write_str("").unwrap();
    t.child("tests/lib.cairo").write_str("").unwrap();

    let metadata = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version=1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let test_cu: Vec<(PathBuf, String)> = metadata
        .compilation_units
        .iter()
        .filter(|cu| cu.target.kind == "test")
        .map(|cu| {
            (
                cu.target.source_path.clone().into_std_path_buf(),
                cu.target
                    .params
                    .as_object()
                    .unwrap()
                    .get("test-type")
                    .unwrap()
                    .to_string(),
            )
        })
        .sorted()
        .collect();

    assert_eq!(
        test_cu,
        vec![
            (
                fsx::canonicalize(t.child("src/lib.cairo")).unwrap(),
                r#""unit""#.into()
            ),
            (
                fsx::canonicalize(t.child("tests/lib.cairo")).unwrap(),
                r#""integration""#.into()
            ),
        ]
    );
}

#[test]
fn can_choose_target_by_name() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .dep_cairo_test()
        .dep_starknet()
        .manifest_extra(indoc! {r#"
            [lib]
            [[target.starknet-contract]]
        "#})
        .lib_cairo(r#"fn f() -> felt252 { 42 }"#)
        .build(&t);
    t.child("tests").create_dir_all().unwrap();
    t.child("tests/test1.cairo")
        .write_str(indoc! {r#"
        #[cfg(test)]
        mod tests {
            use hello::f;
            #[test]
            fn it_works() {
                assert(f() == 42, 'it works!');
            }
        }
         "#})
        .unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .arg("--target-names=hello")
        .current_dir(&t)
        .assert()
        .success();

    assert_eq!(t.child("target").files(), vec!["CACHEDIR.TAG", "dev"]);
    assert_eq!(
        t.child("target/dev").files(),
        vec![
            ".fingerprint",
            "hello.sierra.json",
            "hello.starknet_artifacts.json",
            "incremental"
        ]
    );
}

#[test]
fn can_choose_target_by_kind() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .dep_cairo_test()
        .dep_starknet()
        .manifest_extra(indoc! {r#"
            [lib]
            [[target.starknet-contract]]
        "#})
        .lib_cairo(r#"fn f() -> felt252 { 42 }"#)
        .build(&t);
    t.child("tests").create_dir_all().unwrap();
    t.child("tests/test1.cairo")
        .write_str(indoc! {r#"
        #[cfg(test)]
        mod tests {
            use hello::f;
            #[test]
            fn it_works() {
                assert(f() == 42, 'it works!');
            }
        }
         "#})
        .unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .arg("--target-kinds=lib")
        .current_dir(&t)
        .assert()
        .success();

    assert_eq!(t.child("target").files(), vec!["CACHEDIR.TAG", "dev"]);
    assert_eq!(
        t.child("target/dev").files(),
        vec![".fingerprint", "hello.sierra.json", "incremental"]
    );
}

#[test]
fn cannot_use_both_test_and_target_kind() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .dep_cairo_test()
        .dep_starknet()
        .manifest_extra(indoc! {r#"
            [lib]
            [[target.starknet-contract]]
        "#})
        .lib_cairo(r#"fn f() -> felt252 { 42 }"#)
        .build(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--test")
        .arg("--target-kinds=lib")
        .current_dir(&t)
        .assert()
        .failure()
        .stderr_matches(indoc! {r#"
            error: the argument '--test' cannot be used with '--target-kinds <TARGET_KINDS>'

            Usage: scarb[EXE] build --test

            For more information, try '--help'.
        "#});
}

#[test]
fn cannot_use_both_target_names_and_target_kind() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .dep_cairo_test()
        .dep_starknet()
        .manifest_extra(indoc! {r#"
            [lib]
            [[target.starknet-contract]]
        "#})
        .lib_cairo(r#"fn f() -> felt252 { 42 }"#)
        .build(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--target-names=hello")
        .arg("--target-kinds=lib")
        .current_dir(&t)
        .assert()
        .failure()
        .stderr_matches(indoc! {r#"
            error: the argument '--target-names <TARGET_NAMES>' cannot be used with '--target-kinds <TARGET_KINDS>'

            Usage: scarb[EXE] build --target-names <TARGET_NAMES>

            For more information, try '--help'.
        "#});
}

#[test]
fn can_use_test_and_target_names() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .dep_cairo_test()
        .dep_starknet()
        .manifest_extra(indoc! {r#"
            [lib]
            [[target.starknet-contract]]
        "#})
        .lib_cairo(r#"fn f() -> felt252 { 42 }"#)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("check")
        .arg("--test")
        .arg("--target-names=hello")
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn test_target_builds_contracts() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [lib]
            sierra = true

            [[target.starknet-contract]]
        "#})
        .dep_starknet()
        .dep_cairo_test()
        .lib_cairo(indoc! {r#"
            pub mod balance;
            pub mod forty_two;
        "#})
        .src("src/balance.cairo", BALANCE_CONTRACT)
        .src("src/forty_two.cairo", FORTY_TWO_CONTRACT)
        .build(&t);

    t.child("tests/contract_test.cairo")
        .write_str(
            formatdoc! {r#"
        #[cfg(test)]
        mod tests {{

            {HELLO_CONTRACT}

            use array::ArrayTrait;
            use core::result::ResultTrait;
            use core::traits::Into;
            use option::OptionTrait;
            use starknet::syscalls::deploy_syscall;
            use traits::TryInto;

            use hello::balance::{{Balance, IBalance, IBalanceDispatcher, IBalanceDispatcherTrait}};

            #[test]
            fn test_flow() {{
                let calldata = array![100];
                let (address0, _) = deploy_syscall(
                    Balance::TEST_CLASS_HASH.try_into().unwrap(), 0, calldata.span(), false
                )
                    .unwrap();
                let mut contract0 = IBalanceDispatcher {{ contract_address: address0 }};

                let calldata = array![200];
                let (address1, _) = deploy_syscall(
                    Balance::TEST_CLASS_HASH.try_into().unwrap(), 0, calldata.span(), false
                )
                    .unwrap();
                let mut contract1 = IBalanceDispatcher {{ contract_address: address1 }};

                assert_eq!(@contract0.get(), @100, "contract0.get() == 100");
                assert_eq!(@contract1.get(), @200, "contract1.get() == 200");
                @contract1.increase(200);
                assert_eq!(@contract0.get(), @100, "contract0.get() == 100");
                assert_eq!(@contract1.get(), @400, "contract1.get() == 400");
            }}
        }}
    "#}
            .as_str(),
        )
        .unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .arg("--test")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]Compiling test(hello_unittest) hello v0.1.0 ([..]Scarb.toml)
        [..]Compiling test(hello_integrationtest) hello_integrationtest v0.1.0 ([..]Scarb.toml)
        [..]  Finished `dev` profile target(s) in [..]
        "#});

    assert_eq!(
        t.child("target/dev").files(),
        vec![
            ".fingerprint",
            "hello_integrationtest.test.json",
            "hello_integrationtest.test.sierra.json",
            "hello_integrationtest.test.starknet_artifacts.json",
            "hello_integrationtest_Balance.test.contract_class.json",
            "hello_integrationtest_FortyTwo.test.contract_class.json",
            "hello_integrationtest_HelloContract.test.contract_class.json",
            "hello_unittest.test.json",
            "hello_unittest.test.sierra.json",
            "hello_unittest.test.starknet_artifacts.json",
            "hello_unittest_Balance.test.contract_class.json",
            "hello_unittest_FortyTwo.test.contract_class.json",
            "incremental",
        ]
    );

    for json in [
        "hello_integrationtest_Balance.test.contract_class.json",
        "hello_integrationtest_FortyTwo.test.contract_class.json",
        "hello_integrationtest_HelloContract.test.contract_class.json",
        "hello_unittest_Balance.test.contract_class.json",
        "hello_unittest_FortyTwo.test.contract_class.json",
    ] {
        t.child("target/dev")
            .child(json)
            .assert_is_json::<ContractClass>();
    }

    t.child("target/dev/hello_integrationtest.test.starknet_artifacts.json")
        .assert_is_json::<serde_json::Value>();
    t.child("target/dev/hello_unittest.test.starknet_artifacts.json")
        .assert_is_json::<serde_json::Value>();
}

#[test]
fn test_target_builds_external() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [lib]
            [[target.starknet-contract]]
        "#})
        .dep_starknet()
        .dep_cairo_test()
        .lib_cairo(HELLO_CONTRACT)
        .build(&t.child("first"));

    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [lib]
            sierra = true

            [[target.starknet-contract]]
            build-external-contracts = ["first::*"]
        "#})
        .dep("first", Dep.path("../first"))
        .dep_starknet()
        .dep_cairo_test()
        .build(&t.child("hello"));

    Scarb::quick_snapbox()
        .arg("build")
        .arg("--test")
        .current_dir(t.child("hello"))
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]Compiling test(hello_unittest) hello v0.1.0 ([..]Scarb.toml)
        [..]  Finished `dev` profile target(s) in [..]
        "#});

    assert_eq!(
        t.child("hello/target/dev").files(),
        vec![
            ".fingerprint",
            "hello_unittest.test.json",
            "hello_unittest.test.sierra.json",
            "hello_unittest.test.starknet_artifacts.json",
            "hello_unittest_HelloContract.test.contract_class.json",
            "incremental",
        ]
    );

    t.child("hello/target/dev/hello_unittest_HelloContract.test.contract_class.json")
        .assert_is_json::<ContractClass>();

    t.child("hello/target/dev/hello_unittest.test.starknet_artifacts.json")
        .assert_is_json::<serde_json::Value>();
}

#[test]
fn transitive_dev_deps_not_available() {
    let t = TempDir::new().unwrap();

    let first = &t.child("first");
    ProjectBuilder::start()
        .lib_cairo(indoc! {r#"
            pub fn forty_two() -> felt252 { 42 }
        "#})
        .name("first")
        .build(first);
    let second = &t.child("second");
    ProjectBuilder::start()
        .name("second")
        .dep("first", first)
        .build(second);
    let hello = t.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
            use first::forty_two;
            pub fn main() -> felt252 { forty_two() }
        "#})
        .dep("second", second)
        .dev_dep("first", first)
        .build(&hello);

    Scarb::quick_snapbox()
        .arg("check")
        .current_dir(hello)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            [..]Checking hello v1.0.0 ([..]Scarb.toml)
            error[E0006]: Identifier not found.
             --> [..]lib.cairo:1:5
            use first::forty_two;
                ^^^^^

            error: could not check `hello` due to previous error
        "#});
}

#[test]
fn test_executable_compiler_creates_output_files() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("executable_test")
        .dep_cairo_test()
        .dep_starknet()
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [[target.executable]]

            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 {
                42
            }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target/dev/executable_test.executable.json")
        .assert(predicates::path::exists());
}

#[test]
fn compile_executable_target_can_use_short_declaration() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("executable_test")
        .dep_cairo_test()
        .dep_starknet()
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]

            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 {
                42
            }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target/dev/executable_test.executable.json")
        .assert(predicates::path::exists());
}

#[test]
fn executable_target_requires_disabled_gas() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("executable_test")
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

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..]Compiling executable_test v1.0.0 ([..]Scarb.toml)
        error: executable target cannot be compiled with enabled gas calculation
        help: if you want to disable gas calculation, consider adding following
        excerpt to your package manifest
            -> Scarb.toml
                [cairo]
                enable-gas = false
        error: could not compile `executable_test` due to previous error
        "#});

    t.child("target/dev/executable_test.executable.json")
        .assert(predicates::path::exists().not());
}

#[test]
fn compile_executable_with_missing_plugin() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("executable_test")
        .manifest_extra(indoc! {r#"
            [executable]

            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 {
                42
            }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..]Compiling executable_test v1.0.0 ([..]Scarb.toml)
        warn: package `executable_test` declares `executable` target, but does not depend on `cairo_execute` package
        note: this may cause contract compilation to fail with cryptic errors
        help: add dependency on `cairo_execute` to package manifest
         --> Scarb.toml
            [dependencies]
            cairo_execute = "[..]"
        
        error: Plugin diagnostic: Unsupported attribute.
         --> [..]lib.cairo:1:1
        #[executable]
        ^^^^^^^^^^^^^
        
        error: could not compile `executable_test` due to previous error
        "#});

    t.child("target/dev/executable_test.executable.json")
        .assert(predicates::path::exists().not());
}

#[test]
fn executable_for_multiple_functions() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .dep_cairo_test()
        .dep_starknet()
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]
            function = "hello_world::main"

            [[target.executable]]
            name = "secondary"
            function = "hello_world::secondary"

            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 {
                42
            }

            #[executable]
            fn secondary() -> felt252 {
                42
            }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target/dev/hello_world.executable.json")
        .assert(predicates::path::exists());
    t.child("target/dev/secondary.executable.json")
        .assert(predicates::path::exists());
}

#[test]
fn ambiguous_executable_function() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .dep_cairo_test()
        .dep_starknet()
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]

            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 {
                42
            }

            #[executable]
            fn secondary() -> felt252 {
                42
            }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
               Compiling hello_world v1.0.0 ([..]Scarb.toml)
            error: more than one executable found in the main crate:
            [..]hello_world::main
            [..]hello_world::secondary
            help: specify a separate `executable` target for each of your executable functions
            -> Scarb.toml
            [[target.executable]]
            name = "main"
            function = "hello_world::main"

            [[target.executable]]
            name = "secondary"
            function = "hello_world::secondary"
            error: could not compile `hello_world` due to previous error
        "#});
}

#[test]
fn test_target_builds_contracts_with_error() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [lib]
            sierra = true

            [[target.starknet-contract]]
        "#})
        .dep_starknet()
        .dep_cairo_test()
        .lib_cairo(indoc! {r#"
            pub mod hello;
        "#})
        .src(
            "src/hello.cairo",
            indoc! {r#"
            #[starknet::interface]
            trait IHelloContract<T> {
                fn answer(ref self: T) -> felt252;
            }
            #[starknet::contract]
            mod HelloContract {
                #[storage]
                struct Storage {}
                #[abi(embed_v0)]
                impl HelloContract of super::IHelloContract<ContractState> {
                    fn answer(ref self: ContractState) -> felt252 { boo() }
                }
            }
        "#},
        )
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .arg("--test")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            [..]Compiling test(hello_unittest) hello v0.1.0 ([..]Scarb.toml)
            error[E0006]: Function not found.
             --> [..]hello.cairo:11:57
                    fn answer(ref self: ContractState) -> felt252 { boo() }
                                                                    ^^^

            error: could not compile `hello` due to previous error
        "#});
}

#[test]
fn test_target_builds_contracts_with_warning() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [lib]
            sierra = true

            [[target.starknet-contract]]
        "#})
        .dep_starknet()
        .dep_cairo_test()
        .lib_cairo(indoc! {r#"
            pub mod hello;
            pub mod fibmod;
        "#})
        .src(
            "src/fibmod.cairo",
            indoc! {r#"
            pub fn fib(mut n: u32) -> u32 {
                let mut a: u32 = 0;
                let mut b: u32 = 1;
                while n != 0 {
                    n = n - 1;
                    let temp = b;
                    b = a + b;
                    a = temp;
                };
                a
            }
        "#},
        )
        .src(
            "src/hello.cairo",
            indoc! {r#"
            use hello::fibmod::fib;

            #[starknet::interface]
            trait IHelloContract<T> {
                fn answer(ref self: T) -> felt252;
            }
            #[starknet::contract]
            mod HelloContract {
                #[storage]
                struct Storage {}
                #[abi(embed_v0)]
                impl HelloContract of super::IHelloContract<ContractState> {
                    fn answer(ref self: ContractState) -> felt252 { 'hello' }
                }
            }
        "#},
        )
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .arg("--test")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Compiling test(hello_unittest) hello v0.1.0 ([..]Scarb.toml)
            warn: Unused import: `hello::hello::fib`
             --> [..]hello.cairo:1:20
            use hello::fibmod::fib;
                               ^^^

                Finished `dev` profile target(s) in [..]
        "#});

    assert_eq!(
        t.child("target/dev").files(),
        vec![
            ".fingerprint",
            "hello_unittest.test.json",
            "hello_unittest.test.sierra.json",
            "hello_unittest.test.starknet_artifacts.json",
            "hello_unittest_HelloContract.test.contract_class.json",
            "incremental",
        ]
    );
}

#[test]
fn executable_target_validation() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .dep_cairo_test()
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]
            name = "first"

            [[target.executable]]
            name = "second"
            function = "hello_world::a"

            [[target.executable]]
            name = "third"
            function = "hello_world::b"

            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(indoc! {r#"
            #[executable]
            fn a() -> felt252 {
                12
            }
            #[executable]
            fn b() -> felt252 {
                34
            }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            warn: you have specified multiple executable targets
            some of them specify different `function` names, some do not specify `function` name at all
            this is probably a mistake
            if your project defines more than one executable function, you need to specify `function` name


            [..]Compiling executable(first) [..] v1.0.0 ([..]Scarb.toml)
            error: more than one executable found in the main crate:
                pkg0::a
            	pkg0::b
            help: specify a separate `executable` target for each of your executable functions
            -> Scarb.toml
            [[target.executable]]
            name = "a"
            function = "pkg0::a"

            [[target.executable]]
            name = "b"
            function = "pkg0::b"
            error: could not compile `pkg0` due to previous error
        "#});
}

#[test]
fn disallowed_test_target_names() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .dep_cairo_test()
        .lib_cairo(r#"fn f() -> felt252 { 42 }"#)
        .build(&t);
    t.child("tests").create_dir_all().unwrap();
    t.child("tests/hint.cairo")
        .write_str(indoc! {r#"
        #[cfg(test)]
        mod tests {
            use hello::f;
            #[test]
            fn it_works() {
                assert(f() == 42, 'it works!');
            }
        }
         "#})
        .unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .arg("--test")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: failed to parse manifest at: [..]/Scarb.toml

        Caused by:
            the name `hint` cannot be used as a test target name, names cannot use Cairo keywords see the full list at https://starknet.io/cairo-book/appendix-01-keywords.html consider renaming file: [..]
        "#});
}

#[test]
fn test_target_defaults() {
    let t = TempDir::new().unwrap();
    let hello = t.child("hello");
    let world = t.child("world");

    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [lib]
            [[target.starknet-contract]]
        "#})
        .dep_starknet()
        .lib_cairo(format!("{BALANCE_CONTRACT}\n{HELLO_CONTRACT}"))
        .build(&hello);

    ProjectBuilder::start()
        .name("world")
        .edition("2023_01")
        .version("0.1.0")
        .dep("hello", Dep.path("../hello"))
        .manifest_extra(formatdoc! {r#"
            [[test]]
            name = "a"
            path = "tests/a.cairo"
            build-external-contracts = [
                "hello::HelloContract",
            ]

            [[test]]
            name = "b"
            path = "tests/b.cairo"

            [target-defaults.test]
            build-external-contracts = [
                "hello::Balance",
            ]
        "#})
        .dep_starknet()
        .build(&world);

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version", "1"])
        .current_dir(&world)
        .stdout_json::<Metadata>();

    let world_package = metadata
        .packages
        .iter()
        .find(|p| p.name == "world")
        .unwrap();
    let (mut a, mut b) = (None, None);
    world_package
        .targets
        .iter()
        .for_each(|t| match t.name.as_str() {
            "a" => a = Some(t),
            "b" => b = Some(t),
            _ => {}
        });

    assert_eq!(
        a.unwrap().params,
        json!({
            "build-external-contracts": ["hello::HelloContract"],
            "path": "tests/a.cairo"
        })
    );

    assert_eq!(
        b.unwrap().params,
        json!({
            "build-external-contracts": ["hello::Balance"],
            "path": "tests/b.cairo"
        })
    );
}

#[test_case(
    r#"
         [target-defaults.test]
         build-external-contracts.workspace = true
         "#
)]
#[test_case(
    r#"
            [target-defaults]
            test.workspace = true
        "#
)]
fn test_workspace_target_defaults_param(target_defaults: &str) {
    let t = TempDir::new().unwrap();
    let hello = t.child("hello");

    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [lib]
            [[target.starknet-contract]]
        "#})
        .dep_starknet()
        .lib_cairo(format!("{BALANCE_CONTRACT}\n{HELLO_CONTRACT}"))
        .build(&hello);

    WorkspaceBuilder::start()
        .add_member("hello")
        .manifest_extra(formatdoc! {r#"
            [package]
            name = "world"
            version = "0.1.0"

            [workspace.target-defaults.test]
            build-external-contracts = [
                "hello::Balance",
            ]

            {}

            [[test]]
            name = "a"
            path = "tests/a.cairo"
            build-external-contracts = [
              "hello::HelloContract",
            ]

            [[test]]
            name = "b"
            path = "tests/b.cairo"
            "#, target_defaults
        })
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version", "1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let world_package = metadata
        .packages
        .iter()
        .find(|p| p.name == "world")
        .unwrap();
    let (mut a, mut b) = (None, None);
    world_package
        .targets
        .iter()
        .for_each(|t| match t.name.as_str() {
            "a" => a = Some(t),
            "b" => b = Some(t),
            _ => {}
        });

    assert_eq!(
        a.unwrap().params,
        json!({
            "build-external-contracts": ["hello::HelloContract"],
            "path": "tests/a.cairo"
        })
    );

    assert_eq!(
        b.unwrap().params,
        json!({
            "build-external-contracts": ["hello::Balance"],
            "path": "tests/b.cairo"
        })
    );
}

#[test]
fn test_target_defaults_fails_for_unsupported_target_kind() {
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [lib]

            [target-defaults.lib]
            build-external-contracts = [
                "hello::Balance",
            ]

        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("check")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
error: failed to parse manifest at: [..]/Scarb.toml

Caused by:
    TOML parse error at line 10, column 18
       |
    10 | [target-defaults.lib]
       |                  ^^^
    only target kind `test` is allowed in `target_defaults`, but found `lib`
"#});
}

#[test]
fn test_target_defaults_overrides_auto_detected_targets() {
    let t = TempDir::new().unwrap();
    let hello = t.child("hello");
    let world = t.child("world");

    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [lib]
            [[target.starknet-contract]]
        "#})
        .dep_starknet()
        .lib_cairo(format!("{BALANCE_CONTRACT}\n{HELLO_CONTRACT}"))
        .build(&hello);

    ProjectBuilder::start()
        .name("world")
        .edition("2023_01")
        .version("0.1.0")
        .dep("hello", Dep.path("../hello"))
        .manifest_extra(formatdoc! {r#"
            [target-defaults.test]
            build-external-contracts = [
                "hello::Balance",
            ]
        "#})
        .dep_starknet()
        .build(&world);

    world.child("tests").create_dir_all().unwrap();
    world.child("tests/test1.cairo");

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version", "1"])
        .current_dir(&world)
        .stdout_json::<Metadata>();

    metadata
        .packages
        .iter()
        .for_each(|p| match p.name.as_str() {
            "world" => {
                p.targets
                    .iter()
                    .filter(|t| t.kind == "test")
                    .for_each(|t| match t.name.as_str() {
                        "world_test1" => assert_eq!(
                            t.params,
                            json!({
                                "build-external-contracts": ["world::*", "hello::Balance"],
                                "test-type": "integration",
                                "group-id": "world_integrationtest",
                            })
                        ),
                        "world_unittest" => assert_eq!(
                            t.params,
                            json!({
                                "build-external-contracts": ["hello::Balance"],
                                "test-type": "unit"
                            })
                        ),
                        _ => panic!("unexpected test target"),
                    });
            }
            _ => p.targets.iter().for_each(|t| {
                assert_ne!(
                    t.params.get("build-external-contracts"),
                    Some(&json!(["hello::Balance"]))
                );
            }),
        });
}

#[test]
fn can_compile_executable_to_sierra() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("executable_test")
        .dep_cairo_test()
        .dep_starknet()
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]
            name = "main"
            function = "executable_test::main"
            sierra = true

            [[target.executable]]
            name = "second"
            function = "executable_test::second"
            sierra = true

            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 {
                42
            }
            #[executable]
            fn second() -> felt252 {
                42
            }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target/dev/main.executable.sierra.json")
        .assert(predicates::str::is_empty().not());
    t.child("target/dev/second.executable.sierra.json")
        .assert(predicates::str::is_empty().not());
}
