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
        .edition("2023_01")
        .lib_cairo(indoc! {r#"
            #[starknet::contract]
            pub(crate) mod DualCaseERC20Mock 
            }
          
        "#})
        .dep_starknet()
        .build(&t);

    let output = Scarb::quick_snapbox()
        .arg("doc")
        .current_dir(&t)
        .assert()
        .failure();

    failure_assert(
        output,
        indoc! {r#"
            error: Expected either ';' or '{' after module name. Use ';' for an external module declaration or '{' for a module with a body.
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

#[test]
fn test_diagnostics_warnings() {
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello_world")
        .edition("2023_01")
        .lib_cairo(indoc! {r#"
            #[doc(group = "wrong syntax")]
            fn wrong_syntax() {}
            
            #[doc(group: "wrong syntax2")]
            fn wrong_syntax2() {}
            
            #[doc(wrong_argument_name: 'group name')]
            fn wrong_argument_name() {}
            
            #[doc(hiddens)]
            fn typo() {}
            
            #[doc(hidden)]
            fn correct_doc_hidden() {}
        "#})
        .dep_starknet()
        .build(&t);

    Scarb::quick_snapbox()
        .arg("doc")
        .args(["--document-private-items", "--output-format", "json"])
        .current_dir(&t)
        .assert()
        .stdout_matches(indoc! {r#"
            warn: Invalid attribute `#doc(group = "wrong syntax")]` in hello_world::wrong_syntax.
            Use `#[doc(group: 'group name')]'` or `#[doc(hidden)]`, instead
            warn: Invalid attribute `group: "wrong syntax2"` in hello_world::wrong_syntax2.
            Use `group: 'group name'` instead.
            warn: Invalid attribute `wrong_argument_name: 'group name'` in hello_world::wrong_argument_name.
            Use `group: 'group name'` instead.
            warn: Invalid attribute `#doc(hiddens)]` in hello_world::typo.
            Use `#[doc(group: 'group name')]'` or `#[doc(hidden)]`, instead
            Saving output to: target/doc/output.json
        "#});
}
