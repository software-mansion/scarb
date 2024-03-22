use assert_fs::TempDir;
use indoc::indoc;
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
            [..]Finished release target(s) in [..]
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
        .build(&t);
    Scarb::quick_snapbox()
        .arg("cairo-test")
        .arg("--print-resource-usage")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Compiling test(hello_unittest) hello v1.0.0 ([..]Scarb.toml)
            [..]Finished release target(s) in [..]
            testing hello ...
            running 1 test
            test hello::tests::it_works ... ok (gas usage est.: [..])
                steps: [..]
                memory holes: [..]
                builtins: ("range_check_builtin": [..])
            test result: ok. 1 passed; 0 failed; 0 ignored; 0 filtered out;

        "#});
}
