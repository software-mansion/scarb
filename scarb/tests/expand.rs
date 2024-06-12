use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
use indoc::{formatdoc, indoc};
use itertools::Itertools;
use scarb_test_support::command::Scarb;
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn expand_package_simple() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
            fn hello() -> felt252 {
                0
            }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("expand")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("");

    assert_eq!(t.child("target").files(), vec!["CACHEDIR.TAG", "dev"]);
    assert_eq!(t.child("target/dev").files(), vec!["hello.expanded.cairo"]);
    let expanded = t.child("target/dev/hello.expanded.cairo").read_to_string();
    snapbox::assert_eq(
        indoc! {r#"
        mod hello {
            fn hello() -> felt252 {
                0
            }
        }
        "#},
        expanded,
    );
}

#[test]
fn expand_integration_test() {
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
        .src("tests/a.cairo", test_case)
        .src("tests/b.cairo", test_case)
        .build(&t);
    Scarb::quick_snapbox()
        .arg("expand")
        .arg("--target-name=hello_a")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("");
    assert_eq!(
        t.child("target/dev").files(),
        vec!["hello_a.expanded.cairo"]
    );
    let expanded = t
        .child("target/dev/hello_a.expanded.cairo")
        .read_to_string();
    snapbox::assert_eq(
        indoc! {r#"
        mod hello_integrationtest {
            mod a {
                mod tests {
                    use hello::fib;

                    #[test]
                    fn it_works() {
                        assert(fib(16) == 987, 'it works!');
                    }
                }
            }
        }
        "#},
        expanded,
    );
}

#[test]
fn can_select_target_by_kind() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
            fn hello() -> felt252 {
                42
            }

            #[cfg(test)]
            mod tests {
                use super::hello;

                #[test]
                fn it_works() {
                    assert(hello() == 42, 'it works!');
                }
            }
        "#})
        .build(&t);
    Scarb::quick_snapbox()
        .arg("expand")
        .arg("--target-kind=test")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("");
    assert_eq!(
        t.child("target/dev").files(),
        vec!["hello_unittest.expanded.cairo"]
    );
    let expanded = t
        .child("target/dev/hello_unittest.expanded.cairo")
        .read_to_string();
    snapbox::assert_eq(
        indoc! {r#"
            mod hello {
                fn hello() -> felt252 {
                    42
                }

                mod tests {
                    use super::hello;

                    #[test]
                    fn it_works() {
                        assert(hello() == 42, 'it works!');
                    }
                }
            }
        "#},
        expanded,
    );
}

#[test]
fn can_expand_multiple_targets() {
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
        .src("tests/a.cairo", test_case)
        .src("tests/b.cairo", test_case)
        .build(&t);
    Scarb::quick_snapbox()
        .arg("expand")
        .arg("--target-kind=test")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("");
    assert_eq!(
        t.child("target/dev")
            .files()
            .into_iter()
            .sorted()
            .collect_vec(),
        vec![
            "hello_a.expanded.cairo",
            "hello_b.expanded.cairo",
            "hello_unittest.expanded.cairo",
        ]
    );
    let expanded = t
        .child("target/dev/hello_unittest.expanded.cairo")
        .read_to_string();
    snapbox::assert_eq(
        indoc! {r#"
            mod hello {
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

                mod tests {
                    use hello::fib;

                    #[test]
                    fn it_works() {
                        assert(fib(16) == 987, 'it works!');
                    }
                }
            }
        "#},
        expanded,
    );
    let expanded = t
        .child("target/dev/hello_a.expanded.cairo")
        .read_to_string();
    snapbox::assert_eq(
        indoc! {r#"
        mod hello_integrationtest {
            mod a {
                mod tests {
                    use hello::fib;

                    #[test]
                    fn it_works() {
                        assert(fib(16) == 987, 'it works!');
                    }
                }
            }
        }
        "#},
        expanded,
    );
    let expanded = t
        .child("target/dev/hello_b.expanded.cairo")
        .read_to_string();
    snapbox::assert_eq(
        indoc! {r#"
        mod hello_integrationtest {
            mod b {
                mod tests {
                    use hello::fib;

                    #[test]
                    fn it_works() {
                        assert(fib(16) == 987, 'it works!');
                    }
                }
            }
        }
        "#},
        expanded,
    );
}

#[test]
fn selected_target_must_exist() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
            fn hello() -> felt252 {
                42
            }
        "#})
        .build(&t);
    Scarb::quick_snapbox()
        .arg("expand")
        .arg("--target-kind=non-existent")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq("error: no compilation units found for `hello`\n");
}

#[test]
fn attempts_formatting() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
            fn hello() -> felt252 {


            42}
            #[cfg(test)]
            mod tests {
                use super::hello;
                #[test]
                fn it_works() {


                assert(hello() == 42, 'it works!');
                                }
            }
        "#})
        .build(&t);
    Scarb::quick_snapbox()
        .arg("expand")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("");
    assert_eq!(t.child("target/dev").files(), vec!["hello.expanded.cairo"]);
    let expanded = t.child("target/dev/hello.expanded.cairo").read_to_string();
    // Defaults to lib target - hence no tests (stripped by config plugin).
    snapbox::assert_eq(
        indoc! {r#"
            mod hello {
                fn hello() -> felt252 {
                    42
                }
            }
        "#},
        expanded,
    );
}

#[test]
fn can_skip_formatting() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
            fn hello() -> felt252 {


            42}
        "#})
        .build(&t);
    Scarb::quick_snapbox()
        .arg("expand")
        .arg("--ugly")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("");
    assert_eq!(t.child("target/dev").files(), vec!["hello.expanded.cairo"]);
    let expanded = t.child("target/dev/hello.expanded.cairo").read_to_string();
    snapbox::assert_eq(
        indoc! {r#"

            mod hello {
            fn hello() -> felt252 {


            42}
            }
        "#},
        expanded,
    );
}

#[test]
fn can_expand_erroneous_code() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        // Missing opening bracket.
        .lib_cairo(indoc! {r#"
            fn hello() -> felt252 {
                0

        "#})
        .build(&t);
    Scarb::quick_snapbox()
        .arg("expand")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            error: Missing token TerminalRBrace.
             --> [..]lib.cairo:2:6
                0
                 ^

        "#});
    assert_eq!(t.child("target").files(), vec!["CACHEDIR.TAG", "dev"]);
    assert_eq!(t.child("target/dev").files(), vec!["hello.expanded.cairo"]);
    let expanded = t.child("target/dev/hello.expanded.cairo").read_to_string();
    snapbox::assert_eq(
        indoc! {r#"

        mod hello {
        fn hello() -> felt252 {
            0
        }
        "#},
        expanded,
    );
}
