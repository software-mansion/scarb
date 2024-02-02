use assert_fs::TempDir;
use indoc::indoc;
use scarb_test_support::project_builder::ProjectBuilder;
use snapbox::cmd::{cargo_bin, Command};

fn setup_fib_three_felt_args(t: &TempDir) {
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo(indoc! {r#"
        fn main(a: felt252, b: felt252, n: felt252) -> felt252 {
            fib(a, b, n)
        }

        fn fib(a: felt252, b: felt252, n: felt252) -> felt252 {
            match n {
                0 => a,
                _ => fib(b, a + b, n - 1),
            }
        }
        "#})
        .build(t);
}

#[test]
fn valid_number_of_args() {
    let t = TempDir::new().unwrap();
    setup_fib_three_felt_args(&t);

    Command::new(cargo_bin("scarb"))
        .env("SCARB_TARGET_DIR", t.path())
        .arg("cairo-run")
        .arg("--")
        .arg(r#"[0, 1, 16]"#)
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
               Compiling hello v0.1.0 ([..]/Scarb.toml)
                Finished release target(s) in [..]
                 Running hello
            Run completed successfully, returning [987]
        "#});
}

#[test]
fn invalid_number_of_args() {
    let t = TempDir::new().unwrap();
    setup_fib_three_felt_args(&t);

    Command::new(cargo_bin("scarb"))
        .env("SCARB_TARGET_DIR", t.path())
        .arg("cairo-run")
        .arg("--")
        .arg(r#"[2, 1, 3, 7]"#)
        .current_dir(&t)
        .assert()
        .failure()
        .stderr_matches(indoc! {r#"
            Error: failed to run the function

            Caused by:
                Function expects arguments of size 3 and received 4 instead.
        "#});
}

#[test]
fn array_instead_of_felt() {
    let t = TempDir::new().unwrap();
    setup_fib_three_felt_args(&t);

    Command::new(cargo_bin("scarb"))
        .env("SCARB_TARGET_DIR", t.path())
        .arg("cairo-run")
        .arg("--")
        .arg(r#"[0, 1, [17]]"#)
        .current_dir(&t)
        .assert()
        .failure()
        .stderr_matches(indoc! {r#"
            Error: failed to run the function

            Caused by:
                Function param 2 only partially contains argument 2.
        "#});
}

#[test]
fn invalid_string_instead_of_felt() {
    let t = TempDir::new().unwrap();
    setup_fib_three_felt_args(&t);

    Command::new(cargo_bin("scarb"))
        .env("SCARB_TARGET_DIR", t.path())
        .arg("cairo-run")
        .arg("--")
        .arg(r#"[0, 1, "asdf"]"#)
        .current_dir(&t)
        .assert()
        .failure()
        .stderr_matches(indoc! {r#"
            error: invalid value '[0, 1, "asdf"]' for '[ARGUMENTS]': failed to parse arguments: failed to parse bigint: invalid digit found in string at line 1 column 14

            For more information, try '--help'.
        "#});
}
