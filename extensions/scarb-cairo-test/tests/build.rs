use assert_fs::TempDir;
use indoc::{formatdoc, indoc};
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn can_test_without_gas() {
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


            #[cfg(test)]
            mod tests {
                use super::foo;

                #[test]
                fn test_foo() {
                    foo(array![1, 2].span());
                }
            }
        "#})
        .dep_cairo_test()
        .manifest_extra(indoc! {r#"
            [cairo]
            enable-gas = false
        "#})
        .build(&t);
    Scarb::quick_snapbox()
        .arg("cairo-test")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Compiling test(hello_unittest) hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            testing hello ...
            running 1 test
            test hello::tests::test_foo ... ok
            test result: ok. 1 passed; 0 failed; 0 ignored; 0 filtered out;

        "#});
}

#[test]
fn can_print_test_resources() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
            fn main() -> u32 {
                fib(16)
            }

            fn fib(mut n: u32) -> u32 {
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

            #[cfg(test)]
            mod tests {
                use super::fib;

                #[test]
                fn it_works() {
                    assert(fib(16) == 987, 'it works!');
                }
            }
        "#})
        .dep_cairo_test()
        .build(&t);
    Scarb::quick_snapbox()
        .arg("cairo-test")
        .arg("--print-resource-usage")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Compiling test(hello_unittest) hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            testing hello ...
            running 1 test
            test hello::tests::it_works ... ok (gas usage est.: [..])
                steps: [..]
                memory holes: [..]
                builtins: ("range_check_builtin": [..])
            test result: ok. 1 passed; 0 failed; 0 ignored; 0 filtered out;

        "#});
}

fn get_features_test_build(t: &TempDir) {
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
            [features]
            x = []
            "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn f() -> felt252 { 21 }

            fn main() -> felt252 { f() }

            #[cfg(test)]
            mod tests {
                use super::main;

                #[test]
                fn it_works() {
                    assert(main() == 21, 'it works!');
                }
            }
        "#})
        .dep_cairo_test()
        .build(t);
}

#[test]
fn features_test_build_success() {
    let t = TempDir::new().unwrap();
    get_features_test_build(&t);
    Scarb::quick_snapbox()
        .arg("test")
        .arg("--features=x")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Running cairo-test hello
            [..]Compiling test(hello_unittest) hello v1.0.0 ([..])
            [..]Finished `dev` profile target(s) in [..]
            testing hello ...
            running 1 test
            test hello::tests::it_works ... ok[..]
            test result: ok. 1 passed; 0 failed; 0 ignored; 0 filtered out;

        "#});
}

#[test]
fn features_test_build_failed() {
    let t = TempDir::new().unwrap();
    get_features_test_build(&t);
    let snapbox = Scarb::quick_snapbox()
        .arg("cairo-test")
        .current_dir(&t)
        .assert()
        .failure();

    #[cfg(not(windows))]
    snapbox.stdout_matches(indoc! {r#"
        [..]Compiling test(hello_unittest) hello v1.0.0 ([..])
        error: Function not found.
         --> [..]/src/lib.cairo[..]
        fn main() -> felt252 { f() }
                               ^
        
        error: could not compile `hello` due to previous error[..]
    "#});
    #[cfg(windows)]
    snapbox.stdout_matches(indoc! {r#"
        [..]Compiling test(hello_unittest) hello v1.0.0 ([..])
        error: Function not found.
         --> [..]/src/lib.cairo[..]
        fn main() -> felt252 { f() }
                               ^
        
        error: could not compile `hello` due to previous error[..]
        error: process did not exit successfully: exit code: 1
    "#});
}

#[test]
fn integration_tests() {
    let t = TempDir::new().unwrap();
    let test_case = indoc! {r#"
        #[cfg(test)]
        mod tests {
            use hello::fib;

            #[test]
            fn it_works() {
                assert(fib(16) == 987, 'it works!');
            }
        }
    "#};
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(formatdoc! {r#"
            fn fib(mut n: u32) -> u32 {{
                let mut a: u32 = 0;
                let mut b: u32 = 1;
                while n != 0 {{
                    n = n - 1;
                    let temp = b;
                    b = a + b;
                    a = temp;
                }};
                a
            }}

            {test_case}
        "#})
        .dep_cairo_test()
        .src("tests/a.cairo", test_case)
        .src("tests/b.cairo", test_case)
        .build(&t);
    Scarb::quick_snapbox()
        .arg("cairo-test")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Compiling test(hello_unittest) hello v1.0.0 ([..]Scarb.toml)
            [..]Compiling test(hello_integrationtest) hello_integrationtest v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            testing hello ...
            running 2 tests
            test hello_integrationtest::[..]::tests::it_works ... ok (gas usage est.: 40740)
            test hello_integrationtest::[..]::tests::it_works ... ok (gas usage est.: 40740)
            test result: ok. 2 passed; 0 failed; 0 ignored; 0 filtered out;

            running 1 test
            test hello::tests::it_works ... ok (gas usage est.: 40740)
            test result: ok. 1 passed; 0 failed; 0 ignored; 0 filtered out;

        "#});
}

#[test]
fn warn_if_cairo_test_plugin_missing() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(formatdoc! {r#"
            fn fib(mut n: u32) -> u32 {{
                let mut a: u32 = 0;
                let mut b: u32 = 1;
                while n != 0 {{
                    n = n - 1;
                    let temp = b;
                    b = a + b;
                    a = temp;
                }};
                a
            }}
        "#})
        .build(&t);
    Scarb::quick_snapbox()
        .arg("cairo-test")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            warn: `cairo_test` plugin not found
            please add the following snippet to your Scarb.toml manifest:
            ```
            [dev-dependencies]
            cairo_test = "[..]"
            ```

            [..]Compiling test(hello_unittest) hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            testing hello ...
            running 0 tests
            test result: ok. 0 passed; 0 failed; 0 ignored; 0 filtered out;

       "#});
}

#[test]
fn can_choose_test_kind_to_run() {
    let t = TempDir::new().unwrap();
    let test_case = indoc! {r#"
        #[cfg(test)]
        mod tests {
            use hello::fib;

            #[test]
            fn it_works() {
                assert(fib(16) == 987, 'it works!');
            }
        }
    "#};
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(formatdoc! {r#"
            fn fib(mut n: u32) -> u32 {{
                let mut a: u32 = 0;
                let mut b: u32 = 1;
                while n != 0 {{
                    n = n - 1;
                    let temp = b;
                    b = a + b;
                    a = temp;
                }};
                a
            }}

            {test_case}
        "#})
        .dep_cairo_test()
        .src("tests/a.cairo", test_case)
        .src("tests/b.cairo", test_case)
        .build(&t);
    Scarb::quick_snapbox()
        .arg("cairo-test")
        .arg("--test-kind")
        .arg("unit")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Compiling test(hello_unittest) hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            testing hello ...
            running 1 test
            test hello::tests::it_works ... ok (gas usage est.: 40740)
            test result: ok. 1 passed; 0 failed; 0 ignored; 0 filtered out;

        "#});
    Scarb::quick_snapbox()
        .arg("cairo-test")
        .arg("--test-kind")
        .arg("integration")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Compiling test(hello_integrationtest) hello_integrationtest v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            testing hello ...
            running 2 tests
            test hello_integrationtest::[..]::tests::it_works ... ok (gas usage est.: 40740)
            test hello_integrationtest::[..]::tests::it_works ... ok (gas usage est.: 40740)
            test result: ok. 2 passed; 0 failed; 0 ignored; 0 filtered out;
            
        "#});
}
