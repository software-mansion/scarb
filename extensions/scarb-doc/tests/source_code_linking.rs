use assert_fs::TempDir;
use assert_fs::fixture::PathChild;
use indoc::indoc;
use scarb_test_support::{command::Scarb, project_builder::ProjectBuilder};
use std::path::PathBuf;

use scarb_test_support::workspace_builder::WorkspaceBuilder;
use walkdir::WalkDir;

fn format_expected_url(remote_base_url: &str, root_dir_name: &str, subpath: &str) -> String {
    format!("<a href='{remote_base_url}{root_dir_name}{subpath}'> [source code] </a>")
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
                .join("target/doc/src/a_package-a_submodule.md"),
            root_dir
                .path()
                .join("target/doc/src/a_package-free_functions.md"),
            root_dir.path().join("target/doc/src/a_package-main.md"),
            root_dir.path().join("target/doc/src/a_package-modules.md"),
            root_dir.path().join("target/doc/src/a_package.md"),
            root_dir.path().join("target/doc/src/b_package-SomeEnum.md"),
            root_dir.path().join("target/doc/src/b_package-enums.md"),
            root_dir.path().join("target/doc/src/b_package.md"),
        ]
    );

    // assert links are correct
    let root_dir_name = root_dir.path().file_name().unwrap().to_str().unwrap();
    let pairs = vec![
        ("a_package.md", "/a_package/src/lib.cairo"),
        ("a_package-a_submodule.md", "/a_package/src/lib.cairo#L5-L8"),
        ("a_package-main.md", "/a_package/src/lib.cairo#L1-L3"),
        ("b_package.md", "/b_package/src/lib.cairo"),
        ("b_package-SomeEnum.md", "/b_package/src/lib.cairo#L1-L7"),
    ];
    for (doc_file, subpath) in pairs {
        let content =
            std::fs::read_to_string(root_dir.path().join("target/doc/src").join(doc_file)).unwrap();

        let expected_link = format_expected_url(remote_base_url, root_dir_name, subpath);
        assert!(content.contains(&expected_link));
    }
}

const MACROS_CODE: &str = include_str!("code/code_11.cairo");

#[test]
fn expanded_macros_links() {
    let root_dir = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello_world")
        .edition("2023_11")
        .lib_cairo(MACROS_CODE)
        .manifest_package_extra(r#"experimental-features = ["user_defined_inline_macros"]"#)
        .build(&root_dir);

    let remote_base_url = "https://github.com/ExampleRepoOwner/ExampleRepoProject/blob/master/";

    Scarb::quick_command()
        .arg("doc")
        .args(["--remote-base-url", remote_base_url])
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
            root_dir.path().join("target/doc/hello_world/book.toml"),
            root_dir
                .path()
                .join("target/doc/hello_world/src/SUMMARY.md"),
            root_dir
                .path()
                .join("target/doc/hello_world/src/exposed_can_be_a_part_of_a_group-traits.md"),
            root_dir
                .path()
                .join("target/doc/hello_world/src/exposed_can_be_a_part_of_a_group.md"),
            root_dir
                .path()
                .join("target/doc/hello_world/src/hello_world-OuterStruct.md"),
            root_dir
                .path()
                .join("target/doc/hello_world/src/hello_world-ShapeShifters.md"),
            root_dir
                .path()
                .join("target/doc/hello_world/src/hello_world-define_function.md"),
            root_dir
                .path()
                .join("target/doc/hello_world/src/hello_world-define_trait.md"),
            root_dir
                .path()
                .join("target/doc/hello_world/src/hello_world-free_functions.md"),
            root_dir
                .path()
                .join("target/doc/hello_world/src/hello_world-macro_declarations.md"),
            root_dir
                .path()
                .join("target/doc/hello_world/src/hello_world-modules.md"),
            root_dir
                .path()
                .join("target/doc/hello_world/src/hello_world-my_macro_defined_function.md"),
            root_dir
                .path()
                .join("target/doc/hello_world/src/hello_world-nested_module_macro.md"),
            root_dir
                .path()
                .join("target/doc/hello_world/src/hello_world-regina-VisibleStruct.md"),
            root_dir
                .path()
                .join("target/doc/hello_world/src/hello_world-regina-structs.md"),
            root_dir
                .path()
                .join("target/doc/hello_world/src/hello_world-regina.md"),
            root_dir
                .path()
                .join("target/doc/hello_world/src/hello_world-structs.md"),
            root_dir
                .path()
                .join("target/doc/hello_world/src/hello_world.md"),
            // when panics with hello::secret_mod::secret_fn and hello::not_public::not_a_public_macro
            // create ad use fn new_with_file_link_data for scarb_doc::types::module_type::ModulePubUses
        ]
    );

    // assert links are correct
    let root_dir_name = root_dir.path().file_name().unwrap().to_str().unwrap();
    let pairs = vec![
        ("hello_world.md", "/src/lib.cairo"),
        ("hello_world-define_function.md", "/src/lib.cairo#L16-L22"),
        ("hello_world-define_trait.md", "/src/lib.cairo#L26-L35"),
        (
            "hello_world-my_macro_defined_function.md",
            "/src/lib.cairo#L24-L24",
        ),
        (
            "hello_world-nested_module_macro.md",
            "/src/lib.cairo#L44-L70",
        ),
        ("hello_world-OuterStruct.md", "/src/lib.cairo#L80-L83"),
        ("hello_world-regina.md", "/src/lib.cairo#L72-L72"),
        (
            "hello_world-regina-VisibleStruct.md",
            "/src/lib.cairo#L72-L72",
        ),
        ("hello_world-ShapeShifters.md", "/src/lib.cairo#L36-L36"),
    ];
    for (doc_file, subpath) in pairs {
        let content = std::fs::read_to_string(
            root_dir
                .path()
                .join("target/doc/hello_world/src")
                .join(doc_file),
        )
        .unwrap();
        let expected_link = format_expected_url(remote_base_url, root_dir_name, subpath);
        assert!(content.contains(&expected_link));
    }
}

#[test]
fn special_characters_in_base_url_are_not_malformed() {
    let root_dir = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("sp_chars")
        .edition("2024_07")
        .lib_cairo(indoc! {r#"
            pub fn main() {}
        "#})
        .build(&root_dir);

    let remote_base_url =
        "https://git.example.com/acme%20space/repo/blob/feat/%23branch(plus+sign)/";

    Scarb::quick_command()
        .arg("doc")
        .env("REMOTE_BASE_URL", remote_base_url)
        .current_dir(&root_dir)
        .assert()
        .success();

    let root_dir_name = root_dir.path().file_name().unwrap().to_str().unwrap();

    let content = std::fs::read_to_string(
        root_dir
            .path()
            .join("target/doc/sp_chars/src/sp_chars-main.md"),
    )
    .unwrap();

    let expected_link = format_expected_url(remote_base_url, root_dir_name, "/src/lib.cairo#L1-L1");

    assert!(content.contains(&expected_link));
}
