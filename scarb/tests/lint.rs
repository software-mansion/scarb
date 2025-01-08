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
        .stdout_matches(indoc! {r#"
              Checking hello v1.0.0 ([..]Scarb.toml)
          warning: Plugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.
           --> [..]
            |
          3 |     if x == false {
            |        ----------
            |
        "#})
        .success();
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
      .stdout_matches(indoc! {r#"
            Checking first v1.0.0 ([..]first/Scarb.toml)
        warning: Plugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.
         --> [..]/lib.cairo:3:8
          |
        3 |     if first == false {
          |        --------------
          |
            Checking main v1.0.0 ([..]/Scarb.toml)
        warning: Plugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.
         --> [..]/lib.cairo:3:8
          |
        3 |     if _main == false {
          |        --------------
          |
            Checking second v1.0.0 ([..]second/Scarb.toml)  
        warning: Plugin diagnostic: Unnecessary comparison with a boolean value. Use the variable directly.
         --> [..]/lib.cairo:3:8
          |
        3 |     if second == false {
          |        ---------------
          |          
      "#}).success();
}
