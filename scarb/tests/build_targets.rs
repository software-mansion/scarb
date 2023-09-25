use assert_fs::prelude::*;
use assert_fs::TempDir;
use indoc::indoc;
use itertools::Itertools;
use predicates::prelude::*;
use scarb_metadata::Metadata;
use std::path::PathBuf;

use scarb_test_support::command::{CommandExt, Scarb};
use scarb_test_support::fsx;
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn compile_with_duplicate_targets_1() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"

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
        [..]  Finished release target(s) in [..]
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
        [..]  Finished release target(s) in [..]
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
        [..]  Finished release target(s) in [..]
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
        .arg("build") // TODO(#137): Change build to check for faster and lighter test.
        .current_dir(&hello)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            warn: hello v1.0.0 ([..]) ignoring invalid dependency `dep` which is missing a lib or cairo-plugin target
               Compiling hello v1.0.0 ([..])
            error: Identifier not found.
             --> [..]/lib.cairo:1:25
            fn hellp() -> felt252 { dep::forty_two() }
                                    ^*^


            error: could not compile `hello` due to previous error
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
    assert_eq!(t.child("target/dev").files(), vec!["hello.sierra.json"]);
}

#[test]
fn compile_test_target() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(r#"fn f() -> felt252 { 42 }"#)
        .build(&t);
    t.child("tests").create_dir_all().unwrap();
    t.child("tests/test1.cairo")
        .write_str(indoc! {r#"
        #[cfg(test)]
        mod tests {
            use hello::f;
            #[test]
            #[available_gas(100000)]
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
            "hello.sierra.json",
            "hello_unittest.test.json",
            "test1.test.json"
        ]
    );
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
