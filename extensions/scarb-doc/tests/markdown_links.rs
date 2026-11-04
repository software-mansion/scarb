//! Run `UPDATE_EXPECT=1 cargo test` to fix the tests.

use assert_fs::TempDir;
use indoc::indoc;
use scarb_test_support::workspace_builder::WorkspaceBuilder;
mod markdown_target;
use markdown_target::MarkdownTargetChecker;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

const EXPECTED_LINKED_ITEMS: &str = "tests/data/hello_world_linked_items";
const LINKED_ITEMS_CODE: &str = include_str!("code/code_6.cairo");

#[test]
fn test_markdown_linked_items() {
    let root_dir = TempDir::new().unwrap();

    let root = ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(LINKED_ITEMS_CODE);

    WorkspaceBuilder::start().package(root).build(&root_dir);

    Scarb::quick_command()
        .arg("doc")
        .args(["--document-private-items", "--disable-remote-linking"])
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
        .expected(EXPECTED_LINKED_ITEMS)
        .assert_all_files_match();
}

#[test]
fn test_markdown_external_links() {
    let root_dir = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
            /// Uses the [`shell` package](https://scarbs.xyz/packages/shell) and [Scarb](https://docs.swmansion.com/scarb).
            pub fn main() {}
        "#})
        .build(&root_dir);

    Scarb::quick_command()
        .arg("doc")
        .arg("--disable-remote-linking")
        .current_dir(&root_dir)
        .assert()
        .success();

    let doc_dir = root_dir.path().join("target/doc/hello_world/src");
    let expected = [
        "[`shell` package](https://scarbs.xyz/packages/shell)",
        "[Scarb](https://docs.swmansion.com/scarb)",
    ];
    for file in ["hello_world-main.md", "hello_world.md"] {
        let content = std::fs::read_to_string(doc_dir.join(file)).unwrap();
        for link in expected {
            assert!(
                content.contains(link),
                "`{link}` not found in {file}:\n{content}"
            );
        }
    }
}
