use assert_fs::prelude::*;
use predicates::prelude::*;
use snapbox::cmd::{cargo_bin, Command};

#[test]
fn compile_simple() {
    let t = assert_fs::TempDir::new().unwrap();
    t.child("Murek.toml")
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

    Command::new(cargo_bin!("murek"))
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
    t.child("Murek.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();
    t.child("src/lib.cairo")
        .write_str(r"invalid syntax")
        .unwrap();

    Command::new(cargo_bin!("murek"))
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stderr_eq("Error: Compilation failed.\n");
}

#[test]
fn compile_without_manifest() {
    let t = assert_fs::TempDir::new().unwrap();
    Command::new(cargo_bin!("murek"))
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stderr_matches(
            "\
Error: failed to read manifest at `[..]/Murek.toml`

Caused by:
    No such file or directory (os error 2)
",
        );
}

#[test]
#[cfg(target_os = "linux")]
fn compile_with_lowercase_murek_toml() {
    let t = assert_fs::TempDir::new().unwrap();
    t.child("murek.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();
    Command::new(cargo_bin!("murek"))
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stderr_matches(
            "\
Error: failed to read manifest at `[..]/Murek.toml`

Caused by:
    No such file or directory (os error 2)
",
        );
}

#[test]
fn compile_with_manifest_not_a_file() {
    let t = assert_fs::TempDir::new().unwrap();
    t.child("Murek.toml").create_dir_all().unwrap();
    Command::new(cargo_bin!("murek"))
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stderr_matches(
            "\
Error: failed to read manifest at `[..]/Murek.toml`

Caused by:
    Is a directory (os error 21)
",
        );
}

#[test]
fn compile_with_invalid_empty_name() {
    let t = assert_fs::TempDir::new().unwrap();
    t.child("Murek.toml")
        .write_str(
            r#"
            [package]
            name = ""
            version = "0.1.0"
            "#,
        )
        .unwrap();
    Command::new(cargo_bin!("murek"))
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stderr_matches(
            "\
Error: failed to parse manifest at `[..]/Murek.toml`

Caused by:
    empty string cannot be used as package name
",
        );
}

#[test]
fn compile_with_invalid_version() {
    let t = assert_fs::TempDir::new().unwrap();
    t.child("Murek.toml")
        .write_str(
            r#"
            [package]
            name = "foo"
            version = "y"
            "#,
        )
        .unwrap();
    Command::new(cargo_bin!("murek"))
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stderr_matches(
            "\
Error: failed to parse manifest at `[..]/Murek.toml`

Caused by:
    unexpected character 'y' while parsing major version number for key `package.version`
",
        );
}

#[test]
fn compile_with_invalid_non_numeric_dep_version() {
    let t = assert_fs::TempDir::new().unwrap();
    t.child("Murek.toml")
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
    Command::new(cargo_bin!("murek"))
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stderr_matches(
            "\
Error: failed to parse manifest at `[..]/Murek.toml`

Caused by:
    data did not match any variant of untagged enum TomlDependency for key `dependencies.moo`
",
        );
}

#[test]
fn compile_multiple_packages() {
    let t = assert_fs::TempDir::new().unwrap();

    t.child("Murek.toml")
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

    t.child("decrement/Murek.toml")
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

    Command::new(cargo_bin!("murek"))
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("")
        .stderr_eq("");

    t.child("target/release/fib.sierra")
        .assert(predicates::str::is_empty().not());
}

#[test]
fn compile_with_nested_deps() {
    let t = assert_fs::TempDir::new().unwrap();

    t.child("Murek.toml")
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

    t.child("y/Murek.toml")
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

    t.child("z/Murek.toml")
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

    t.child("q/Murek.toml")
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

    Command::new(cargo_bin!("murek"))
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("")
        .stderr_eq("");

    t.child("target/release/x.sierra")
        .assert(predicates::str::is_empty().not());
}
