use assert_fs::TempDir;
use indoc::indoc;
use scarb_test_support::{command::Scarb, project_builder::ProjectBuilder};

mod json_target;
use json_target::JsonTargetChecker;

#[test]
fn test_doc_groups() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
          #[doc(group: 'test')]
          fn test_2() {
              println!('test2');
          }
          #[doc(group: 'test2')]
          fn test_1() {
              println!('test1');
          }
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
        .expected("./data/json_doc_groups.json")
        .assert_files_match();
}
