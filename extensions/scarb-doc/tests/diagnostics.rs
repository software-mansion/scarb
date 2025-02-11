use assert_fs::TempDir;
use indoc::indoc;
use scarb_test_support::{command::Scarb, project_builder::ProjectBuilder};
use snapbox::cmd::OutputAssert;

#[test]
fn test_diagnostics_success() {
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
      fn main() {
        println!("Hello world!");
      }
    "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("doc")
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn test_diagnostics_with_error_code() {
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
          fn main() {
            println!("Hello world!");
            wrong code
          }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("doc")
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn test_diagnostics_allowed_warnings() {
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
          fn main() {
            println!("Hello world!");
            let a = 5;
          }
        "#})
        .manifest_extra(indoc! {r#"
        [cairo]
        allow-warnings = true
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("doc")
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn test_diagnostics_not_allowed_warnings() {
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
          fn main() {
            println!("Hello world!");
            let a = 5;
          }
        "#})
        .manifest_extra(indoc! {r#"
        [cairo]
        allow-warnings = false
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("doc")
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn test_diagnostics_error() {
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
            #[starknet::contract]
            pub(crate) mod DualCaseERC20Mock 
            }
          
        "#})
        .build(&t);

    let output = Scarb::quick_snapbox()
        .arg("doc")
        .current_dir(&t)
        .assert()
        .failure();

    failure_assert(
        output,
        indoc! {r#"
            error: Missing token TerminalSemicolon.
             --> [..]lib.cairo:2:33
            pub(crate) mod DualCaseERC20Mock 
                                            ^
            
            error: Skipped tokens. Expected: Const/Enum/ExternFunction/ExternType/Function/Impl/InlineMacro/Module/Struct/Trait/TypeAlias/Use or an attribute.
             --> [..]lib.cairo:3:1
            }
            ^
            
            error: Plugin diagnostic: Contracts without body are not supported.
             --> [..]lib.cairo:1:1-2:32
              #[starknet::contract]
             _^
            | pub(crate) mod DualCaseERC20Mock 
            |________________________________^
            
            error[E0005]: Module file not found. Expected path: [..]DualCaseERC20Mock.cairo
             --> [..]lib.cairo:1:1-2:32
              #[starknet::contract]
             _^
            | pub(crate) mod DualCaseERC20Mock 
            |________________________________^
            
            error: Compilation failed.
        "#},
    );
}

fn failure_assert(output: OutputAssert, expected: &str) {
    #[cfg(windows)]
    output.stdout_matches(format!(
        "{expected}error: process did not exit successfully: exit code: 1\n"
    ));
    #[cfg(not(windows))]
    output.stdout_matches(expected);
}
