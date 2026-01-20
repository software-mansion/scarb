use assert_fs::TempDir;
use scarb_test_support::{command::Scarb, project_builder::ProjectBuilder};

mod markdown_target;
use markdown_target::MarkdownTargetChecker;
mod json_target;
use json_target::JsonTargetChecker;
use scarb_test_support::workspace_builder::WorkspaceBuilder;

const DOC_GROUPS_REEKSPORTS_CODE: &str = include_str!("code/code_8.cairo");
const EXPECTED_DOC_GROUPS_REEKSPORTS_PATH: &str = "tests/data/hello_world_doc_groups_reeksports";

#[test]
fn doc_groups_reeksports_json() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(DOC_GROUPS_REEKSPORTS_CODE)
        .build(&t);

    Scarb::quick_command()
        .arg("doc")
        .args(["--output-format", "json"])
        .current_dir(&t)
        .assert()
        .success();

    JsonTargetChecker::default()
        .actual(&t.path().join("target/doc/output.json"))
        .expected("./data/json_doc_groups_reeksports.json")
        .assert_files_match();
}

#[test]
fn doc_groups_reeksports_markdown() {
    let root_dir = TempDir::new().unwrap();
    let root = ProjectBuilder::start()
        .name("hello_world")
        .edition("2023_11")
        .lib_cairo(DOC_GROUPS_REEKSPORTS_CODE);

    WorkspaceBuilder::start().package(root).build(&root_dir);

    Scarb::quick_command()
        .args(["doc", "--disable-remote-linking"])
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
        .expected(EXPECTED_DOC_GROUPS_REEKSPORTS_PATH)
        .assert_all_files_match();
}

#[test]
fn doc_groups_reeksports_markdown_doesnt_duplicate_groups() {
    let root_dir = TempDir::new().unwrap();
    let root = ProjectBuilder::start()
        .name("hello_world")
        .edition("2023_11")
        .lib_cairo(DOC_GROUPS_REEKSPORTS_CODE);

    WorkspaceBuilder::start().package(root).build(&root_dir);

    Scarb::quick_command()
        .arg("doc")
        .args([
            "--document-private-items",
            "--build",
            "--disable-remote-linking",
        ])
        .current_dir(&root_dir)
        .assert()
        .success();
}
