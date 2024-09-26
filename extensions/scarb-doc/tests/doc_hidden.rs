use assert_fs::TempDir;
use indoc::indoc;
use scarb_test_support::{command::Scarb, project_builder::ProjectBuilder};

mod json_target;
use json_target::JsonTargetChecker;

#[test]
fn test_doc_hidden() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
          #[doc(hidden)]
          /// test2
          fn test_2() {
              //! inner comment
              println!("test2");
          }

          /// test1
          #[doc(hidden)]
          fn test_1() {
              //! test comment
              println!("test1");
          }


          /// main
          fn main() {
              println!("hellow")
          }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("doc")
        .args(["--document-private-items", "--output-format", "json"])
        .current_dir(&t)
        .assert()
        .success();

    JsonTargetChecker::default()
        .actual(&t.path().join("target/doc/output.json"))
        .expected("./data/json_doc_hidden.json")
        .assert_files_match();
}
