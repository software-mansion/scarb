use assert_fs::TempDir;
use assert_fs::fixture::PathChild;
use indoc::indoc;
use scarb_test_support::{command::Scarb, project_builder::ProjectBuilder};

use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::workspace_builder::WorkspaceBuilder;

fn format_expected_url(remote_base_url: &str, subpath: &str) -> String {
    format!("<a href='{remote_base_url}{subpath}'> [source code] </a>")
}

#[test]
fn links_workspace() {
    let root_dir = TempDir::new().unwrap();

    let a_child_dir = root_dir.child("a_package");
    let b_child_dir = root_dir.child("b_package");

    ProjectBuilder::start()
        .name("a_package")
        .lib_cairo(indoc! {r#"
              pub fn main() {
                println!("hellow")
              }

              pub mod a_submodule {
              // estra
              // offset
              }
        "#})
        .edition("2024_07")
        .build(&a_child_dir);

    ProjectBuilder::start()
        .name("b_package")
        .edition("2024_07")
        .lib_cairo(indoc! {r#"
        pub enum SomeEnum {
            // some comment that should be included
            // .
            // .
            // .
            // in souce code link lines offest
            }
        "#})
        .build(&b_child_dir);

    WorkspaceBuilder::start()
        .add_member("a_package")
        .add_member("b_package")
        .build(&root_dir);

    let remote_base_url = "https://github.com/ExampleRepoOwner/ExampleRepoProject/blob/master/";

    Scarb::quick_command()
        .arg("doc")
        .arg("--workspace")
        .env("SCARB_DOC_REMOTE_BASE_URL", remote_base_url)
        .current_dir(&root_dir)
        .assert()
        .success();

    let actual_files = root_dir.child("target/doc/src").files();

    // assert files exist
    assert_eq!(
        actual_files,
        vec![
            "SUMMARY.md".to_string(),
            "a_package-a_submodule.md".to_string(),
            "a_package-free_functions.md".to_string(),
            "a_package-main.md".to_string(),
            "a_package-modules.md".to_string(),
            "a_package.md".to_string(),
            "b_package-SomeEnum.md".to_string(),
            "b_package-enums.md".to_string(),
            "b_package.md".to_string(),
        ]
    );

    // assert links are correct
    let pairs = vec![
        ("a_package.md", "a_package/src/lib.cairo"),
        ("a_package-a_submodule.md", "a_package/src/lib.cairo#L5-L8"),
        ("a_package-main.md", "a_package/src/lib.cairo#L1-L3"),
        ("b_package.md", "b_package/src/lib.cairo"),
        ("b_package-SomeEnum.md", "b_package/src/lib.cairo#L1-L7"),
    ];
    for (doc_file, subpath) in pairs {
        let content =
            std::fs::read_to_string(root_dir.path().join("target/doc/src").join(doc_file)).unwrap();

        let expected_link = format_expected_url(remote_base_url, subpath);
        assert!(content.contains(&expected_link));
    }
}
