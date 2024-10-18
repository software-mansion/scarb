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

#[test]
fn hides_impls_of_private_traits() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
          #[doc(hidden)
          struct HiddenStuct {}

          #[doc(hidden)]
          trait HiddenTrait<T> {}

          struct VisibleStruct {}

          trait VisibleTrait<T> {}

          impl VisibleImpl of VisibleTrait<VisibleStruct> {}

          impl FirstHiddenImpl of HiddenTrait<HiddenStruct> {}
          impl SecondHiddenImpl of HiddenTrait<VisibleStruct> {}
          impl ThirdHiddenImpl of VisibleTrait<HiddenStruct> {}

          #[doc(hidden)]
          impl FourthHiddenImpl of VisibleTrait<VisibleStruct> {}

          trait SecondVisibleTrait<T,Y> {}

          impl SecondVisibleImpl of SecondVisibleTrait<HiddenStruct, VisibleStruct> {}
          impl FifthHiddenImpl of SecondVisibleTrait<HiddenStruct, HiddenStruct> {}
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
        .expected("./data/json_doc_hidden_impls.json")
        .assert_files_match();
}
