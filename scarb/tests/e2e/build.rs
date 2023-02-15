use std::fs;

use assert_fs::prelude::*;
use indoc::indoc;
use predicates::prelude::*;

use crate::support::command::Scarb;
use crate::support::fsx::ChildPathEx;
use crate::support::project_builder::ProjectBuilder;

#[test]
fn compile_simple() {
    // `TempDir::new` creates the directory, while `create_output_dir` does not mark directory as
    // ephemeral if it already exists.
    // Therefore, we use `.child` here to ensure that cache directory does not exist when running.
    let cache_dir = assert_fs::TempDir::new().unwrap().child("c");

    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);

    Scarb::quick_snapbox()
        .env("SCARB_CACHE", cache_dir.path())
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    assert_eq!(t.child("target").files(), vec!["CACHEDIR.TAG", "release"]);
    assert_eq!(t.child("target/release").files(), vec!["hello.sierra"]);

    cache_dir
        .child("registry/core/core/Scarb.toml")
        .assert(predicates::path::exists());
    cache_dir
        .child("CACHEDIR.TAG")
        .assert(predicates::path::exists());
}

#[test]
fn compile_with_syntax_error() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo("not_a_keyword")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(indoc! {r#"
               Compiling hello v0.1.0 ([..])
            error: Skipped tokens. Expected: Module/Use/FreeFunction/ExternFunction/ExternType/Trait/Impl/Struct/Enum or an attribute.
             --> lib.cairo:1:1
            not_a_keyword
            ^***********^


            error: could not compile `hello` due to previous error
        "#});
}

#[test]
fn compile_with_syntax_error_json() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo("not_a_keyword")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("--json")
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(indoc! {r#"
            {"status":"compiling","message":"hello v0.1.0 ([..])"}
            {"type":"diagnostic","message":"error: Skipped tokens. Expected: Module/Use/FreeFunction/ExternFunction/ExternType/Trait/Impl/Struct/Enum or an attribute./n --> lib.cairo:1:1/nnot_a_keyword/n^***********^/n/n"}
            {"type":"error","message":"could not compile `hello` due to previous error"}
        "#});
}

#[test]
fn compile_without_manifest() {
    let t = assert_fs::TempDir::new().unwrap();
    let cause = fs::read(t.child("Scarb.toml")).unwrap_err();
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(format!(
            "\
error: failed to read manifest at `[..]/Scarb.toml`

Caused by:
    {cause}
"
        ));
}

#[test]
#[cfg(target_os = "linux")]
fn compile_with_lowercase_scarb_toml() {
    let t = assert_fs::TempDir::new().unwrap();
    t.child("scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();
    let cause = fs::read(t.child("Scarb.toml")).unwrap_err();
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(format!(
            "\
error: failed to read manifest at `[..]/Scarb.toml`

Caused by:
    {cause}
"
        ));
}

#[test]
fn compile_with_manifest_not_a_file() {
    let t = assert_fs::TempDir::new().unwrap();
    t.child("Scarb.toml").create_dir_all().unwrap();
    let cause = fs::read(t.child("Scarb.toml")).unwrap_err();
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(format!(
            "\
error: failed to read manifest at `[..]/Scarb.toml`

Caused by:
    {cause}
"
        ));
}

#[test]
fn compile_with_invalid_empty_name() {
    let t = assert_fs::TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = ""
            version = "0.1.0"
            "#,
        )
        .unwrap();
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(indoc! {r#"
            error: failed to parse manifest at `[..]/Scarb.toml`

            Caused by:
                TOML parse error at line 3, column 20
                  |
                3 |             name = ""
                  |                    ^^
                empty string cannot be used as package name
        "#});
}

#[test]
fn compile_with_invalid_version() {
    let t = assert_fs::TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "foo"
            version = "y"
            "#,
        )
        .unwrap();
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(indoc! {r#"
            error: failed to parse manifest at `[..]/Scarb.toml`

            Caused by:
                TOML parse error at line 4, column 23
                  |
                4 |             version = "y"
                  |                       ^^^
                unexpected character 'y' while parsing major version number
        "#});
}

#[test]
fn compile_with_invalid_non_numeric_dep_version() {
    let t = assert_fs::TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"

            [dependencies]
            moo = "y"
            "#,
        )
        .unwrap();
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(indoc! {r#"
            error: failed to parse manifest at `[..]/Scarb.toml`

            Caused by:
                TOML parse error at line 7, column 19
                  |
                7 |             moo = "y"
                  |                   ^^^
                data did not match any variant of untagged enum TomlDependency
        "#});
}

#[test]
fn compile_multiple_packages() {
    let t = assert_fs::TempDir::new().unwrap();

    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "fib"
            version = "1.0.0"

            [dependencies]
            decrement = { path = "decrement" }
            "#,
        )
        .unwrap();

    t.child("src/lib.cairo")
        .write_str(
            r#"
            mod sum_two;

            fn fib(a: felt, b: felt, n: felt) -> felt {
                match n {
                    0 => a,
                    _ => fib(b, sum_two::sum_two(a, b), decrement::decrement(n)),
                }
            }
            "#,
        )
        .unwrap();

    t.child("src/sum_two.cairo")
        .write_str(r#"fn sum_two(a: felt, b: felt) -> felt { a + b }"#)
        .unwrap();

    t.child("decrement/Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "decrement"
            version = "1.0.0"
            "#,
        )
        .unwrap();

    t.child("decrement/src/lib.cairo")
        .write_str(
            r#"
            fn decrement(x: felt) -> felt { x - 1 }
            "#,
        )
        .unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..] Compiling fib v1.0.0 ([..])
            [..]  Finished release target(s) in [..]
        "#});

    t.child("target/release/fib.sierra")
        .assert(predicates::str::is_empty().not());
}

#[test]
fn compile_with_nested_deps() {
    let t = assert_fs::TempDir::new().unwrap();

    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "x"
            version = "1.0.0"

            [dependencies]
            y = { path = "y" }
            "#,
        )
        .unwrap();

    t.child("src/lib.cairo")
        .write_str(r"fn f() -> felt { y::f() }")
        .unwrap();

    t.child("y/Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "y"
            version = "1.0.0"

            [dependencies]
            q = { path = "../q" }
            z = { path = "../z" }
            "#,
        )
        .unwrap();

    t.child("y/src/lib.cairo")
        .write_str(r"fn f() -> felt { z::f() }")
        .unwrap();

    t.child("z/Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "z"
            version = "1.0.0"

            [dependencies]
            q = { path = "../q" }
            "#,
        )
        .unwrap();

    t.child("z/src/lib.cairo")
        .write_str(r"fn f() -> felt { q::f() }")
        .unwrap();

    t.child("q/Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "q"
            version = "1.0.0"
            "#,
        )
        .unwrap();

    t.child("q/src/lib.cairo")
        .write_str(r"fn f() -> felt { 42 }")
        .unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..] Compiling x v1.0.0 ([..])
            [..]  Finished release target(s) in [..]
        "#});

    t.child("target/release/x.sierra")
        .assert(predicates::str::is_empty().not());
}

#[test]
fn compile_with_duplicate_targets_1() {
    let t = assert_fs::TempDir::new().unwrap();
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
        error: failed to parse manifest at `[..]/Scarb.toml`

        Caused by:
            manifest contains duplicate target definitions `example`, consider explicitly naming targets with the `name` field
        "#});
}

#[test]
fn compile_with_duplicate_targets_2() {
    let t = assert_fs::TempDir::new().unwrap();
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
        error: failed to parse manifest at `[..]/Scarb.toml`
        
        Caused by:
            manifest contains duplicate target definitions `example (x)`, use different target names to resolve the conflict
        "#});
}

#[test]
fn compile_with_custom_lib_target() {
    let t = assert_fs::TempDir::new().unwrap();
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
            "#,
        )
        .unwrap();
    t.child("src/lib.cairo")
        .write_str(r#"fn f() -> felt { 42 }"#)
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

    t.child("target/release/not_hello.casm")
        .assert(predicates::str::is_empty().not());
    t.child("target/release/not_hello.sierra")
        .assert(predicates::path::exists().not());
    t.child("target/release/hello.sierra")
        .assert(predicates::path::exists().not());
    t.child("target/release/hello.casm")
        .assert(predicates::path::exists().not());
}
