use assert_fs::prelude::PathChild;
use assert_fs::TempDir;
use indoc::indoc;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;
use scarb_test_support::workspace_builder::WorkspaceBuilder;

mod json_target;
use json_target::JsonTargetChecker;

#[test]
fn test_reexports() {
    let root_dir = TempDir::new().unwrap();
    let child_dir = root_dir.child("sub_package");

    ProjectBuilder::start()
        .name("sub_package")
        .lib_cairo(indoc! {r#"
            pub fn display() {
                println!("Hello from the inner module!");
            }
            pub const ABC: u32 = 44;
  
            pub type Type = u32;
  
            pub struct TestStruct {
                abc: u32
            }
  
            pub enum TestEnum {
                Var1
            }
  
            pub trait TestTrait {
                fn test() -> ();
            }
  
            pub impl TestImpl of TestTrait {
                fn test() {
                    println!("test");
                }
            }
  
            pub extern fn extern_function() -> u32 nopanic;
  
            pub extern type ExternalType;
  
            pub mod inside_sub_module {}
          "#})
        .build(&child_dir);

    let root = ProjectBuilder::start()
        .name("hello_world")
        .dep("sub_package", &child_dir)
        .lib_cairo(indoc! {r#"
          pub use sub_package as package;

          mod sub_module;

          fn main() {
              println!("main");
          }
        "#})
        .src(
            "src/sub_module.cairo",
            indoc! {r#"
          mod inner_module {
            pub fn display() {
                println!("Hello from the inner module!");
            }
            pub const ABC: u32 = 44;

            pub type Type = u32;

            pub struct TestStruct {
                abc: u32
            }

            pub enum TestEnum {
                Var1
            }

            pub trait TestTrait {
                fn test() -> ();
            }

            pub impl TestImpl of TestTrait {
                fn test() {
                    println!("test");
                }
            }

            pub extern fn extern_function() -> u32 nopanic;

            pub extern type ExternalType;

            pub mod inside_inner_module {}
          }

          pub use inner_module::display;
          pub use inner_module::ABC;
          pub use inner_module::Type;
          pub use inner_module::TestStruct;
          pub use inner_module::TestEnum;
          pub use inner_module::TestTrait;
          pub use inner_module::TestImpl;
          pub use inner_module::extern_function;
          pub use inner_module::ExternalType;
          pub use inner_module::inside_inner_module;
        "#},
        );

    WorkspaceBuilder::start()
        .add_member("sub_package")
        .package(root)
        .build(&root_dir);

    Scarb::quick_snapbox()
        .arg("doc")
        .args(["--output-format", "json"])
        .current_dir(&root_dir)
        .assert()
        .success();

    JsonTargetChecker::default()
        .actual(&root_dir.path().join("target/doc/output.json"))
        .expected("./data/json_reexports.json")
        .assert_files_match();
}
