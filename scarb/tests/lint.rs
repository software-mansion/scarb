use assert_fs::fixture::FileWriteStr;
use assert_fs::{TempDir, prelude::PathChild};
use indoc::{formatdoc, indoc};
use scarb_test_support::{
    command::Scarb, project_builder::ProjectBuilder, workspace_builder::WorkspaceBuilder,
};

#[test]
fn lint_main_package() {
    let test_code = indoc! {r#"
      use hello::f1;
      #[test]
      fn it_works() {
          let x = true;
          if false == x {
              println!("x is false");
          }
          assert_eq!(1, f1());
      }
    "#};
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(formatdoc! {r#"
          fn main() {{
              let x = true;
              if x == false {{
                  println!("x is false");
              }}
          }}

          // This should not be checked.
          #[cfg(test)]
          mod tests {{
            {test_code}
          }}
        "#})
        .build(&t);

    // We add this one to test that the linting is not run on the test package.
    t.child("tests/test1.cairo").write_str(test_code).unwrap();

    Scarb::quick_snapbox()
        .arg("lint")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
               Linting hello v1.0.0 ([..]/Scarb.toml)
          warn: Plugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.
           --> [..]/lib.cairo:3:8
              if x == false {
                 ^^^^^^^^^^
  
        "#});
}

#[test]
fn lint_warnings_disallowed_manifest() {
    let test_code = indoc! {r#"
      use hello::f1;
      #[test]
      fn it_works() {
          let x = true;
          if false == x {
              println!("x is false");
          }
          assert_eq!(1, f1());
      }
    "#};
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
          [cairo]
          allow-warnings = false
        "#})
        .lib_cairo(formatdoc! {r#"
          fn main() {{
              let x = true;
              if x == false {{
                  println!("x is false");
              }}
          }}

          // This should not be checked.
          #[cfg(test)]
          mod tests {{
            {test_code}
          }}
        "#})
        .build(&t);

    // We add this one to test that the linting is not run on the test package.
    t.child("tests/test1.cairo").write_str(test_code).unwrap();

    Scarb::quick_snapbox()
        .arg("lint")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
               Linting hello v1.0.0 ([..]/Scarb.toml)
          warn: Plugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.
           --> [..]/lib.cairo:3:8
              if x == false {
                 ^^^^^^^^^^
  
          error: lint checking `hello` failed due to previous errors
        "#});
}

#[test]
fn lint_warnings_disallowed_cli() {
    let test_code = indoc! {r#"
      use hello::f1;
      #[test]
      fn it_works() {
          let x = true;
          if false == x {
              println!("x is false");
          }
          assert_eq!(1, f1());
      }
    "#};
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(formatdoc! {r#"
          fn main() {{
              let x = true;
              if x == false {{
                  println!("x is false");
              }}
          }}

          // This should not be checked.
          #[cfg(test)]
          mod tests {{
            {test_code}
          }}
        "#})
        .build(&t);

    // We add this one to test that the linting is not run on the test package.
    t.child("tests/test1.cairo").write_str(test_code).unwrap();

    Scarb::quick_snapbox()
        .arg("lint")
        .arg("--deny-warnings")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
               Linting hello v1.0.0 ([..]/Scarb.toml)
          warn: Plugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.
           --> [..]/lib.cairo:3:8
              if x == false {
                 ^^^^^^^^^^
  
          error: lint checking `hello` failed due to previous errors
        "#});
}

#[test]
fn lint_warnings_disallowed_cli_and_manifest() {
    let test_code = indoc! {r#"
      use hello::f1;
      #[test]
      fn it_works() {
          let x = true;
          if false == x {
              println!("x is false");
          }
          assert_eq!(1, f1());
      }
    "#};
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
          [cairo]
          allow-warnings = false
        "#})
        .lib_cairo(formatdoc! {r#"
          fn main() {{
              let x = true;
              if x == false {{
                  println!("x is false");
              }}
          }}

          // This should not be checked.
          #[cfg(test)]
          mod tests {{
            {test_code}
          }}
        "#})
        .build(&t);

    // We add this one to test that the linting is not run on the test package.
    t.child("tests/test1.cairo").write_str(test_code).unwrap();

    Scarb::quick_snapbox()
        .arg("lint")
        .arg("--deny-warnings")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
               Linting hello v1.0.0 ([..]/Scarb.toml)
          warn: Plugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.
           --> [..]/lib.cairo:3:8
              if x == false {
                 ^^^^^^^^^^
  
          error: lint checking `hello` failed due to previous errors
        "#});
}

#[test]
fn lint_workspace() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
        .lib_cairo(indoc! {r#"
        fn main() {
            let first = true;
            if first == false {
                println!("x is false");
            }
        }
        "#})
        .build(&t.child("first"));
    ProjectBuilder::start()
        .name("second")
        .lib_cairo(indoc! {r#"
        fn main() {
            let second = true;
            if second == false {
                println!("x is false");
            }
        }
        "#})
        .build(&t.child("second"));

    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .package(ProjectBuilder::start().name("main").lib_cairo(indoc! {r#"
        fn main() {
            let _main = true;
            if _main == false {
                println!("x is false");
            }
        }
        "#}))
        .build(&t);

    Scarb::quick_snapbox()
        .arg("lint")
        .arg("--workspace")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
           Linting first v1.0.0 ([..]/first/Scarb.toml)
      warn: Plugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.
       --> [..]/lib.cairo:3:8
          if first == false {
             ^^^^^^^^^^^^^^

           Linting main v1.0.0 ([..]/Scarb.toml)
      warn: Plugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.
       --> [..]/lib.cairo:3:8
          if _main == false {
             ^^^^^^^^^^^^^^

           Linting second v1.0.0 ([..]/second/Scarb.toml)
      warn: Plugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.
       --> [..]/lib.cairo:3:8
          if second == false {
             ^^^^^^^^^^^^^^^

      "#});
}

#[test]
fn lint_integration_tests() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
          pub fn f1() -> u32 {
              42
          }

          fn main() {
              // This is a comment
          }
        "#})
        .dep_cairo_test()
        .build(&t);
    t.child("tests/test1.cairo")
        .write_str(indoc! {r#"
          use hello::f1;
          #[test]
          fn it_works() {
              let x = true;
              if false == x {
                  println!("x is false");
              }
              assert_eq!(1, f1());
          }
        "#})
        .unwrap();

    Scarb::quick_snapbox()
        .arg("lint")
        .arg("-t")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
               Linting hello v1.0.0 ([..]/Scarb.toml)
               Linting test(hello_unittest) hello v1.0.0 ([..]/Scarb.toml)
               Linting test(hello_integrationtest) hello_integrationtest v1.0.0 ([..]/Scarb.toml)
          warn: Plugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.
           --> [..]/tests/test1.cairo:5:8
              if false == x {
                 ^^^^^^^^^^

        "#});
}

#[test]
fn lint_unit_test() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .dep_cairo_test()
        .manifest_extra(
            r#"
          [[test]]
          test-type = "unit"
        "#,
        )
        .lib_cairo(indoc! {r#"
          pub fn f1() -> u32 {
              42
          }

          fn main() {
              // This is a comment
          }

          #[cfg(test)]
          mod tests {
              use hello::f1;
              #[test]
              fn it_works() {
                  let x = true;
                  if false == x {
                      println!("x is false");
                  }
                  assert_eq!(1, f1());
              }
          }
        "#})
        .dep_cairo_test()
        .build(&t);

    Scarb::quick_snapbox()
        .arg("lint")
        .arg("-t")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
               Linting hello v1.0.0 ([..]/Scarb.toml)
               Linting test(hello) hello v1.0.0 ([..]/Scarb.toml)
          warn: Plugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.
           --> [..]/lib.cairo:15:12
                  if false == x {
                     ^^^^^^^^^^

        "#});
}

#[test]
fn lint_no_panics() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
            fn main() {
                panic!("This should not be linted.");
            }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("lint")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches("     Linting hello v1.0.0 ([..]/Scarb.toml)\n");
}

#[test]
fn lint_panics() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
            [tool]
            cairo-lint.panic = true
        "#})
        .lib_cairo(indoc! {r#"
            fn main() {
                panic!("This should not be linted.");
            }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("lint")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
               Linting hello v1.0.0 ([..]/Scarb.toml)
          warn: Plugin diagnostic: Leaving `panic` in the code is discouraged.
           --> [..]/lib.cairo:2:5
              panic!("This should not be linted.");
              ^^^^^

        "#});
}

#[test]
fn lint_selected_features() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
          [features]
          x = []
          y = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'y')]
            fn f() { 
              println!("Just a correct code.");
            }

            #[cfg(feature: 'x')]
            fn f() { 
                let second = true;
                if second == false {
                    println!("x is false");
                }
            }

            fn main() {
                f();
            }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("lint")
        .arg("--features")
        .arg("y")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches("     Linting hello v1.0.0 ([..]/Scarb.toml)\n");

    Scarb::quick_snapbox()
        .arg("lint")
        .arg("--features")
        .arg("x")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! { r#"
               Linting hello v1.0.0 ([..]/Scarb.toml)
          warn: Plugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.
           --> [..]/lib.cairo:9:8
              if second == false {
                 ^^^^^^^^^^^^^^^
        
        "#});
}

#[test]
fn test_missing_feature() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
            fn main() {
                println!("Just a correct code.");
            }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("lint")
        .arg("--features")
        .arg("x")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: none of the selected packages contains `x` feature
            note: to use features, you need to define [features] section in Scarb.toml
        "#});
}

#[test]
fn lint_selected_targets() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
        .lib_cairo(indoc! {r#"
      fn main() {
          let first = true;
          if first == false {
              println!("x is false");
          }
      }
      "#})
        .build(&t.child("first"));
    ProjectBuilder::start()
        .name("second")
        .lib_cairo(indoc! {r#"
      fn main() {
          let second = true;
          if second == false {
              println!("x is false");
          }
      }
      "#})
        .build(&t.child("second"));

    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .package(ProjectBuilder::start().name("main").lib_cairo(indoc! {r#"
      fn main() {
          let _main = true;
          if _main == false {
              println!("x is false");
          }
      }
      "#}))
        .build(&t);

    Scarb::quick_snapbox()
        .arg("lint")
        .arg("--workspace")
        .arg("--target-names=first,second")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
           Linting first v1.0.0 ([..]/first/Scarb.toml)
      warn: Plugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.
       --> [..]/lib.cairo:3:8
          if first == false {
             ^^^^^^^^^^^^^^

           Linting second v1.0.0 ([..]/second/Scarb.toml)
      warn: Plugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.
       --> [..]/lib.cairo:3:8
          if second == false {
             ^^^^^^^^^^^^^^^

      "#});
}

#[test]
fn lint_specific_file() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
            mod other;

            fn main() {
                let x = true;
                if x == false {
                    println!("x is false");
                }
            }
        "#})
        .src(
            "src/other.cairo",
            indoc! {r#"
            fn main() {
                loop {
                    break ();
                }
            }"#},
        )
        .build(&t);

    Scarb::quick_snapbox()
        .arg("lint")
        .arg("src/other.cairo")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
             Linting hello v1.0.0 ([..]/Scarb.toml)
        warn: Plugin diagnostic: unnecessary double parentheses found after break. Consider removing them.
         --> [..]/src/other.cairo:3:9
                break ();
                ^^^^^^^^^

        "#});
}

#[test]
fn lint_specific_directory() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
            mod my_module;

            fn main() {
                let res_val: Result<i32> = Result::Err('err');
                // This is just a variable.
                let _a = match res_val {
                    Result::Ok(x) => Option::Some(x),
                    Result::Err(_) => Option::None,
                };
            }
        "#})
        .src(
            "src/my_module.cairo",
            indoc! {r#"
            mod a;
            mod b;
            mod c;

            fn main() {
                let res_val: Result<i32> = Result::Err('err');
                // This is just a variable.
                let _a = match res_val {
                    Result::Ok(x) => Option::Some(x),
                    Result::Err(_) => Option::None,
                };
            }"#},
        )
        .src(
            "src/my_module/a.cairo",
            indoc! {r#"
            fn a_func() {
                loop {
                    break ();
                }
            }"#},
        )
        .src(
            "src/my_module/b.cairo",
            indoc! {r#"
            fn test_clone_felt252() {
                let a: felt252 = 'hello';
                let _b = a.clone();
            }"#},
        )
        .src(
            "src/my_module/c.cairo",
            indoc! {r#"
            fn c_func() {
                let x = 42;
                let _y = x * 1;
            }"#},
        )
        .build(&t);

    Scarb::quick_snapbox()
        .arg("lint")
        .arg("src/my_module")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
              Linting hello v1.0.0 ([..]/Scarb.toml)
         warn: Plugin diagnostic: unnecessary double parentheses found after break. Consider removing them.
          --> [..]/src/my_module/a.cairo:3:9
                 break ();
                 ^^^^^^^^^
         
         warn: Plugin diagnostic: using `clone` on type which implements `Copy` trait
          --> [..]/src/my_module/b.cairo:3:14
             let _b = a.clone();
                      ^^^^^^^^^

         warn: Plugin diagnostic: This operation doesn't change the value and can be simplified.
          --> [..]/src/my_module/c.cairo:3:14
             let _y = x * 1;
                      ^^^^^

        "#});
}

#[test]
fn lint_non_existing_file() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
            mod other;

            fn main() {
                let x = true;
                if x == false {
                    println!("x is false");
                }
            }
        "#})
        .src(
            "src/other.cairo",
            indoc! {r#"
            fn main() {
                loop {
                    break ();
                }
            }"#},
        )
        .build(&t);

    Scarb::quick_snapbox()
        .arg("lint")
        .arg("wrong.cairo")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: failed to get absolute path of `wrong.cairo`

            Caused by:
                [..]
        "#});
}
