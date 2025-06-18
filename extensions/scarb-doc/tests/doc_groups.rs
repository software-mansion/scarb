use assert_fs::TempDir;
use indoc::indoc;
use scarb_test_support::{command::Scarb, project_builder::ProjectBuilder};

mod markdown_target;
use markdown_target::MarkdownTargetChecker;
mod json_target;
use json_target::JsonTargetChecker;
use scarb_test_support::workspace_builder::WorkspaceBuilder;

#[test]
fn doc_groups_json() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
          #[doc(group: "test")]
          fn test_2() {
              println!("test2");
          }
          #[doc(group: "test2")]
          fn test_1() {
              println!("test1");
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

const DOC_GROUPS_CODE: &str = include_str!("code/code_7.cairo");
const EXPECTED_DOC_GROUPS_PATH: &str = "tests/data/hello_world_doc_groups";

#[test]
fn doc_groups_markdown() {
    let root_dir = TempDir::new().unwrap();

    let root = ProjectBuilder::start()
        .name("hello_world")
        .edition("2023_11")
        .lib_cairo(DOC_GROUPS_CODE);

    WorkspaceBuilder::start().package(root).build(&root_dir);

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
        .expected(EXPECTED_DOC_GROUPS_PATH)
        .assert_all_files_match();
}
