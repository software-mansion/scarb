use assert_fs::TempDir;
use indoc::indoc;
use scarb_test_support::{command::Scarb, project_builder::ProjectBuilder};

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
fn test_diagnostics_error() {
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
        .failure()
        .stdout_matches(indoc! {r#"
          error: Identifier not found.
           [..]
            wrong code
            ^***^

          error: Missing semicolon
           [..]
            wrong code
                 ^

          error: Identifier not found.
           [..]
            wrong code
                  ^**^

          error: Compilation failed.
        "#});
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
        .success()
        .stdout_matches(indoc! {r#"
            warn[E0001]: Unused variable. Consider ignoring by prefixing with `_`.
             [..]
              let a = 5;
                  ^
            
        "#});
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
        .failure()
        .stdout_matches(indoc! {r#"
            warn[E0001]: Unused variable. Consider ignoring by prefixing with `_`.
             [..]
              let a = 5;
                  ^
            
            error: Compilation failed.
        "#});
}
