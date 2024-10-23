use assert_fs::TempDir;
use indoc::indoc;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

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

    Scarb::quick_snapbox()
        .arg("cairo-run")
        .arg("--")
        .arg(r#"[0, 1, 16]"#)
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
               Compiling hello v0.1.0 ([..]/Scarb.toml)
                Finished `dev` profile target(s) in [..]
                 Running hello
            Run completed successfully, returning [987]
        "#});
}

#[test]
fn can_deserialize_big_number() {
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo(indoc! {r#"
        fn main(n: felt252) -> felt252 {
            n
        }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("cairo-run")
        .arg("--")
        .arg(r#"[1129815197211541481934112806673325772687763881719835256646064516195041515616]"#)
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
               Compiling hello v0.1.0 ([..]/Scarb.toml)
                Finished `dev` profile target(s) in [..]
                 Running hello
            Run completed successfully, returning [1129815197211541481934112806673325772687763881719835256646064516195041515616]
        "#});
}

#[test]
#[ignore = "number of args validation removed in cairo-lang-runner cairo#6489"]
fn invalid_number_of_args() {
    let t = TempDir::new().unwrap();
    setup_fib_three_felt_args(&t);

    let snapbox = Scarb::quick_snapbox()
        .arg("cairo-run")
        .arg("--")
        .arg(r#"[0, 1, 2, 3]"#)
        .current_dir(&t)
        .assert()
        .failure();

    #[cfg(windows)]
    snapbox.stdout_matches(indoc! {r#"
               Compiling hello v0.1.0 ([..]/Scarb.toml)
                Finished `dev` profile target(s) in [..]
                 Running hello
            error: failed to run the function

            Caused by:
                Function expects arguments of size 3 and received 4 instead.
            error: process did not exit successfully: exit code: 1
        "#});
    #[cfg(not(windows))]
    snapbox.stdout_matches(indoc! {r#"
               Compiling hello v0.1.0 ([..]/Scarb.toml)
                Finished `dev` profile target(s) in [..]
                 Running hello
            error: failed to run the function

            Caused by:
                Function expects arguments of size 3 and received 4 instead.
        "#});
}

#[test]
fn array_instead_of_felt() {
    let t = TempDir::new().unwrap();
    setup_fib_three_felt_args(&t);

    let snapbox = Scarb::quick_snapbox()
        .arg("cairo-run")
        .arg("--")
        .arg(r#"[0, 1, [17]]"#)
        .current_dir(&t)
        .assert()
        .failure();

    #[cfg(windows)]
    snapbox.stdout_matches(indoc! {r#"
               Compiling hello v0.1.0 ([..]Scarb.toml)
                Finished `dev` profile target(s) in [..]
                 Running hello
            error: failed to run the function

            Caused by:
                Operation failed: 14:0 - 1, offsets cant be negative
            error: process did not exit successfully: exit code: 1
        "#});
    #[cfg(not(windows))]
    snapbox.stdout_matches(indoc! {r#"
               Compiling hello v0.1.0 ([..]Scarb.toml)
                Finished `dev` profile target(s) in [..]
                 Running hello
            error: failed to run the function

            Caused by:
                Operation failed: 14:0 - 1, offsets cant be negative
        "#});
}

#[test]
fn invalid_string_instead_of_felt() {
    let t = TempDir::new().unwrap();
    setup_fib_three_felt_args(&t);

    Scarb::quick_snapbox()
        .arg("cairo-run")
        .arg("--")
        .arg(r#"[0, 1, "asdf"]"#)
        .env("SCARB_LOG", "error")
        .current_dir(&t)
        .assert()
        .failure()
        .stderr_matches(indoc! {r#"
            error: invalid value '[0, 1, "asdf"]' for '[ARGUMENTS]': failed to parse arguments: failed to parse bigint: invalid digit found in string at line 1 column 14

            For more information, try '--help'.
        "#});
}

#[test]
fn struct_deserialization() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo(indoc! {r#"
        #[derive(Debug, Drop)]
        struct InputOne {
            x: felt252,
            y: felt252,
            z: felt252,
        }

        #[derive(Debug, Drop)]
        struct InputTwo {
            w: Array<felt252>,
        }

        #[derive(Drop, PartialEq)]
        struct OutputData {
            x: felt252,
            y: felt252,
            z: felt252,
            sum_w: felt252,
        }

        fn main(a: InputOne, b: InputTwo) -> OutputData {
            f(a, b)
        }

        fn f(a: InputOne, b: InputTwo) -> OutputData {
            let w_span = b.w.span();
            let mut sum_w = 0;
            let mut i = 0;
            loop {
                if i >= w_span.len() {
                    break;
                }
                sum_w += *w_span[i];
                i += 1;
            };
            OutputData { x: a.x, y: a.y, z: a.z, sum_w: sum_w }
        }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("cairo-run")
        .arg("--")
        .arg(r#"[1, 2, 3, [4, 5, 6]]"#)
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
               Compiling hello v0.1.0 ([..]/Scarb.toml)
                Finished `dev` profile target(s) in [..]
                 Running hello
            Run completed successfully, returning [1, 2, 3, 15]
        "#});
}

#[test]
fn invalid_struct_deserialization() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo(indoc! {r#"
        struct InputData {
            x: felt252,
            y: felt252,
            z: felt252,
        }

        #[derive(Drop, PartialEq)]
        struct OutputData {
            x: felt252,
            y: felt252,
            z: felt252,
        }

        fn main(a: InputData) -> InputData {
            a
        }

        fn f(a: InputData) -> OutputData {
            OutputData { x: a.x, y: a.y, z: a.z }
        }
        "#})
        .build(&t);

    let snapbox = Scarb::quick_snapbox()
        .arg("cairo-run")
        .arg("--")
        .arg(r#"[[0, 1, 2]]"#)
        .current_dir(&t)
        .assert()
        .failure();

    // Received 2, because arrays in Cairo are represented as [begin_addr, end_addr]
    #[cfg(windows)]
    snapbox.stdout_matches(indoc! {r#"
               Compiling hello v0.1.0 ([..]Scarb.toml)
                Finished `dev` profile target(s) in [..]
                 Running hello
            error: failed to run the function

            Caused by:
                Couldn't compute operand op1. Unknown value for memory cell 1:12
            error: process did not exit successfully: exit code: 1
        "#});
    #[cfg(not(windows))]
    snapbox.stdout_matches(indoc! {r#"
               Compiling hello v0.1.0 ([..]Scarb.toml)
                Finished `dev` profile target(s) in [..]
                 Running hello
            error: failed to run the function

            Caused by:
                Couldn't compute operand op1. Unknown value for memory cell 1:12
        "#});
}

#[test]
fn can_accept_nested_array() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo(indoc! {r#"
        fn main(a: felt252, b: felt252, n: Array<Array<felt252>>) {
            println!("[{}, {}, {:?}]", a, b, n);
        }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("cairo-run")
        .arg("--")
        .arg(r#"[0, 1, [[17], [1, 15, 3], [42]]]"#)
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]   Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]    Finished `dev` profile target(s) in [..]
        [..]     Running hello
        [..][0, 1, [[17], [1, 15, 3], [42]]]
        [..]Run completed successfully, returning []
        "#});
}

#[test]
fn cannot_set_gas_limit_for_package_with_disabled_gas_calculation() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
            fn foo(mut shape: Span<usize>) -> usize {
                let mut result: usize = 1;

                loop {
                    match shape.pop_front() {
                        Option::Some(item) => { result *= *item; },
                        Option::None => { break; }
                    };
                };

                result
            }

            fn main() -> usize {
                foo(array![1, 2].span())
            }
        "#})
        .manifest_extra(indoc! {r#"
            [lib]
            sierra = true

            [cairo]
            enable-gas = false
        "#})
        .build(&t);
    let output = Scarb::quick_snapbox()
        .arg("cairo-run")
        .arg("--available-gas")
        .arg("10")
        .current_dir(&t)
        .assert()
        .failure();
    #[cfg(windows)]
    output.stdout_eq(indoc! {r#"
            error: gas calculation disabled for package `hello`, cannot define custom gas limit
            error: process did not exit successfully: exit code: 1
        "#});
    #[cfg(not(windows))]
    output.stdout_eq(indoc! {r#"
            error: gas calculation disabled for package `hello`, cannot define custom gas limit
        "#});
}

#[test]
fn can_control_verbosity() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo(indoc! {r#"
            fn main() {
                println!("something");
            }
        "#})
        .build(&t);
    Scarb::quick_snapbox()
        .arg("--quiet")
        .arg("cairo-run")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        something
        "#});
}
