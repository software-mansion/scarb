use std::fs;

use assert_fs::prelude::*;
use indoc::indoc;
use predicates::prelude::*;

use crate::support::command::scarb_command;

#[test]
fn compile_simple() {
    let t = assert_fs::TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();
    t.child("src/lib.cairo")
        .write_str(r#"fn f() -> felt { 42 }"#)
        .unwrap();

    scarb_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();
    t.child("target/release/hello.sierra")
        .assert(predicates::str::is_empty().not());
}

#[test]
fn compile_with_syntax_error() {
    let t = assert_fs::TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();
    t.child("src/lib.cairo")
        .write_str(r"not_a_keyword")
        .unwrap();

    scarb_command()
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
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();
    t.child("src/lib.cairo")
        .write_str(r"not_a_keyword")
        .unwrap();

    scarb_command()
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
    scarb_command()
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
    scarb_command()
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
    scarb_command()
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
    scarb_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(
            "\
error: failed to parse manifest at `[..]/Scarb.toml`

Caused by:
    empty string cannot be used as package name
",
        );
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
    scarb_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(
            "\
error: failed to parse manifest at `[..]/Scarb.toml`

Caused by:
    unexpected character 'y' while parsing major version number for key `package.version`
",
        );
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
    scarb_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(
            "\
error: failed to parse manifest at `[..]/Scarb.toml`

Caused by:
    data did not match any variant of untagged enum TomlDependency for key `dependencies.moo`
",
        );
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

    scarb_command()
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

    scarb_command()
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
