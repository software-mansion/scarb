use assert_fs::TempDir;
use indoc::indoc;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

pub mod markdown_target;
use markdown_target::MarkdownTargetChecker;

#[test]
fn test_duplicated_items_names() {
    let root_dir = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
      mod sub_module;

      fn main() {
        println!("hellow")
      }

      fn unique_function_name() {
        // pass
      }

      fn duplicated_function_name() {
        // pass
      }

      enum DuplicatedEnumName {
        // pass
      }

      trait DuplicatedTraitName {
        // pass
      }
    "#})
        .src(
            "src/sub_module.cairo",
            indoc! {r#"
      fn duplicated_function_name() {
        // pass
      }

      enum DuplicatedEnumName {
        // pass
      }

      trait DuplicatedTraitName {
        // pass
      }
    "#},
        )
        .build(&root_dir);

    Scarb::quick_snapbox()
        .arg("doc")
        .args(["--document-private-items"])
        .current_dir(&root_dir)
        .assert()
        .success();

    MarkdownTargetChecker::default()
        .actual(
            root_dir
                .path()
                .join("target/doc/hello_world")
                .to_str()
                .unwrap(),
        )
        .expected("tests/data/duplicated_item_names")
        .assert_all_files_match();
}
