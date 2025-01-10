use assert_fs::fixture::FileWriteStr;
use assert_fs::{prelude::PathChild, TempDir};
use indoc::indoc;
use scarb_test_support::{
    command::Scarb, project_builder::ProjectBuilder, workspace_builder::WorkspaceBuilder,
};

#[test]
fn lint_main_package() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
      fn main() {
          let x = true;
          if x == false {
              println!("x is false");
          }
      }
    "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("lint")
        .current_dir(&t)
        .assert()
        .success()
        // Current expected values include ANSI color codes because lint has custom renderer.
        .stdout_matches(indoc! {r#"
               Linting hello v1.0.0 ([..]/Scarb.toml)
          [1m[33mwarning[0m: [1mPlugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.[0m
           [1m[94m-->[0m [..]/lib.cairo:3:8
            [1m[94m|[0m
          [1m[94m3 |[0m     if x == false {
            [1m[94m|[0m        [1m[33m----------[0m
            [1m[94m|[0m
  
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
        // Current expected values include ANSI color codes because lint has custom renderer.
        .stdout_matches(indoc! {r#"
           Linting first v1.0.0 ([..]/first/Scarb.toml)
      [1m[33mwarning[0m: [1mPlugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.[0m
       [1m[94m-->[0m [..]/lib.cairo:3:8
        [1m[94m|[0m
      [1m[94m3 |[0m     if first == false {
        [1m[94m|[0m        [1m[33m--------------[0m
        [1m[94m|[0m

           Linting main v1.0.0 ([..]/Scarb.toml)
      [1m[33mwarning[0m: [1mPlugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.[0m
       [1m[94m-->[0m [..]/lib.cairo:3:8
        [1m[94m|[0m
      [1m[94m3 |[0m     if _main == false {
        [1m[94m|[0m        [1m[33m--------------[0m
        [1m[94m|[0m

           Linting second v1.0.0 ([..]/second/Scarb.toml)
      [1m[33mwarning[0m: [1mPlugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.[0m
       [1m[94m-->[0m [..]/lib.cairo:3:8
        [1m[94m|[0m
      [1m[94m3 |[0m     if second == false {
        [1m[94m|[0m        [1m[33m---------------[0m
        [1m[94m|[0m

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
        // Current expected values include ANSI color codes because lint has custom renderer.
        .stdout_matches(indoc! {r#"
               Linting hello v1.0.0 ([..]/Scarb.toml)
               Linting test(hello_unittest) hello v1.0.0 ([..]/Scarb.toml)
               Linting test(hello_integrationtest) hello_integrationtest v1.0.0 ([..]/Scarb.toml)
          [1m[33mwarning[0m: [1mPlugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.[0m
           [1m[94m-->[0m [..]/tests/test1.cairo:5:8
            [1m[94m|[0m
          [1m[94m5 |[0m     if false == x {
            [1m[94m|[0m        [1m[33m----------[0m
            [1m[94m|[0m

        "#});
}
