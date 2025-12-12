use assert_fs::TempDir;
use assert_fs::fixture::PathChild;
use indoc::indoc;
use scarb_test_support::{command::Scarb, project_builder::ProjectBuilder};
use std::path::PathBuf;

use scarb_test_support::workspace_builder::WorkspaceBuilder;
use walkdir::WalkDir;

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
        "#})
        .edition("2024_07")
        .build(&a_child_dir);

    ProjectBuilder::start()
        .name("b_package")
        .edition("2024_07")
        .lib_cairo(indoc! {r#"pub enum SomeEnum {}"#})
        .build(&b_child_dir);

    WorkspaceBuilder::start()
        .add_member("a_package")
        .add_member("b_package")
        .build(&root_dir);

    let remote_base_url = "https://github.com/ExampleRepoOwner/ExampleRepoProject/blob/master/";

    Scarb::quick_command()
        .arg("doc")
        .arg("--workspace")
        .env("REMOTE_BASE_URL", remote_base_url)
        .current_dir(&root_dir)
        .assert()
        .success();

    let actual_files =
        WalkDir::new(root_dir.path().join("target/doc").to_str().unwrap()).sort_by_file_name();

    let actual_files_paths: Vec<PathBuf> = actual_files
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .collect();

    // assert files exist
    assert_eq!(
        actual_files_paths,
        vec![
            root_dir.path().join("target/doc/book.toml"),
            root_dir.path().join("target/doc/src/SUMMARY.md"),
            root_dir
                .path()
                .join("target/doc/src/a_package-free_functions.md"),
            root_dir.path().join("target/doc/src/a_package-main.md"),
            root_dir.path().join("target/doc/src/a_package.md"),
            root_dir.path().join("target/doc/src/b_package-SomeEnum.md"),
            root_dir.path().join("target/doc/src/b_package-enums.md"),
            root_dir.path().join("target/doc/src/b_package.md"),
        ]
    );

    // assert links are correct
    let root_dir_name = root_dir.path().file_name().unwrap().to_str().unwrap();
    let pairs = vec![
        ("a_package-main.md", "a_package/src/lib.cairo"),
        ("b_package-SomeEnum.md", "b_package/src/lib.cairo"),
        ("a_package.md", "a_package/src/lib.cairo"),
        ("b_package.md", "b_package/src/lib.cairo"),
    ];
    for (doc_file, subpath) in pairs {
        let content =
            std::fs::read_to_string(root_dir.path().join("target/doc/src").join(doc_file)).unwrap();

        let expected_link = format!(
            "<a href='{remote_base_url}{root_dir_name}/{subpath}' target='blank'> [source code] </a>"
        );
        assert!(content.contains(&expected_link));
    }
}
