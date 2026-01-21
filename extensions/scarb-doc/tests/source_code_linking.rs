use assert_fs::TempDir;
use assert_fs::fixture::PathChild;
use indoc::indoc;
use scarb_test_support::{command::Scarb, project_builder::ProjectBuilder};

use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::gitx;
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

    let root_dir_name = root_dir.path().file_name().unwrap().to_str().unwrap();
    let remote_base_url = format!(
        "https://github.com/ExampleRepoOwner/ExampleRepoProject/blob/master/{root_dir_name}/"
    );

    Scarb::quick_command()
        .arg("doc")
        .args(["--remote-base-url", &remote_base_url])
        .current_dir(&root_dir)
        .assert()
        .success();

    let actual_files = root_dir.child("target/doc/hello_world/src").files();

    // assert files exist
    assert_eq!(
        actual_files,
        vec![
            "SUMMARY.md".to_string(),
            "exposed_can_be_a_part_of_a_group-traits.md".to_string(),
            "exposed_can_be_a_part_of_a_group.md".to_string(),
            "hello_world-OuterStruct.md".to_string(),
            "hello_world-ShapeShifters.md".to_string(),
            "hello_world-define_function.md".to_string(),
            "hello_world-define_trait.md".to_string(),
            "hello_world-free_functions.md".to_string(),
            "hello_world-macro_declarations.md".to_string(),
            "hello_world-modules.md".to_string(),
            "hello_world-my_macro_defined_function.md".to_string(),
            "hello_world-nested_module_macro.md".to_string(),
            "hello_world-regina-VisibleStruct.md".to_string(),
            "hello_world-regina-structs.md".to_string(),
            "hello_world-regina.md".to_string(),
            "hello_world-structs.md".to_string(),
            "hello_world.md".to_string(),
        ]
    );

    // assert links are correct
    let pairs = vec![
        ("hello_world.md", "src/lib.cairo"),
        ("hello_world-define_function.md", "src/lib.cairo#L16-L22"),
        ("hello_world-define_trait.md", "src/lib.cairo#L26-L35"),
        (
            "hello_world-my_macro_defined_function.md",
            "src/lib.cairo#L24-L24",
        ),
        (
            "hello_world-nested_module_macro.md",
            "src/lib.cairo#L44-L70",
        ),
        ("hello_world-OuterStruct.md", "src/lib.cairo#L80-L83"),
        ("hello_world-regina.md", "src/lib.cairo#L72-L72"),
        (
            "hello_world-regina-VisibleStruct.md",
            "src/lib.cairo#L72-L72",
        ),
        ("hello_world-ShapeShifters.md", "src/lib.cairo#L36-L36"),
    ];
    for (doc_file, subpath) in pairs {
        let content = std::fs::read_to_string(
            root_dir
                .path()
                .join("target/doc/hello_world/src")
                .join(doc_file),
        )
        .unwrap();
        let expected_link = format_expected_url(&remote_base_url, subpath);
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

    let root_dir_name = root_dir.path().file_name().unwrap().to_str().unwrap();
    let remote_base_url = format!(
        "https://git.example.com/acme%20space/repo/blob/feat/%23branch(plus+sign)/{}/",
        root_dir_name
    );

    Scarb::quick_command()
        .arg("doc")
        .env("SCARB_DOC_REMOTE_BASE_URL", remote_base_url.clone())
        .current_dir(&root_dir)
        .assert()
        .success();

    let content = std::fs::read_to_string(
        root_dir
            .path()
            .join("target/doc/sp_chars/src/sp_chars-main.md"),
    )
    .unwrap();
    let expected_link = format_expected_url(&remote_base_url, "src/lib.cairo#L1-L1");
    assert!(content.contains(&expected_link));
}

#[test]
fn uses_manifest_repository() {
    let root_dir = TempDir::new().unwrap();
    root_dir.child(".gitignore");

    gitx::init(&root_dir);

    let child_dir = root_dir.child("hello_world");
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
              pub fn main() {
                println!("hellow")
              }
        "#})
        .manifest_package_extra(indoc! {r#"
            repository ="https://github.com/ExampleRepoOwner/ExampleRepoProject"
        "#})
        .build(&child_dir);

    gitx::commit(&root_dir);
    let repo = gix::discover(root_dir.path()).unwrap();
    let commit_hash = repo.rev_parse_single("HEAD").unwrap().to_string();

    Scarb::quick_command()
        .arg("doc")
        .current_dir(&child_dir)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            Saving output to: target/doc/hello_world

            Run the following to see the results: 
            `mdbook serve target/doc/hello_world`
            (you will need to have mdbook installed)

            Or build html docs by running `scarb doc --build`
        "#});

    let actual_files = child_dir.child("target/doc/hello_world/src").files();

    // assert files exist
    assert_eq!(
        actual_files,
        vec![
            "SUMMARY.md".to_string(),
            "hello_world-free_functions.md".to_string(),
            "hello_world-main.md".to_string(),
            "hello_world.md".to_string(),
        ]
    );

    // assert links are correct
    let pairs = vec![
        ("hello_world.md", "hello_world/src/lib.cairo"),
        ("hello_world-main.md", "hello_world/src/lib.cairo#L1-L3"),
    ];

    let expected_base_url =
        format!("https://github.com/ExampleRepoOwner/ExampleRepoProject/blob/{commit_hash}/");

    for (doc_file, subpath) in pairs {
        let content = std::fs::read_to_string(
            child_dir
                .path()
                .join("target/doc/hello_world/src/")
                .join(doc_file),
        )
        .unwrap();
        let expected_link = format_expected_url(&expected_base_url, subpath);
        assert!(content.contains(&expected_link));
    }
}

#[test]
fn prioritizes_flag_over_manifest_repository() {
    let root_dir = TempDir::new().unwrap();
    root_dir.child(".gitignore");
    gitx::init(&root_dir);

    let child_dir = root_dir.child("hello_world");
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
              pub fn main() {
                println!("hellow")
              }
        "#})
        .manifest_package_extra(indoc! {r#"
            repository ="https://github.com/manifestRepoUrl"
        "#})
        .build(&child_dir);

    gitx::commit(&root_dir);
    gix::discover(root_dir.path()).unwrap();

    let remote_base_url = "https://github.com/flagRepoUrl/hello_world/";
    Scarb::quick_command()
        .arg("doc")
        .args(["--remote-base-url", remote_base_url])
        .current_dir(&child_dir)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        warn: both `--remote-base-url` and manifest repository URL provided, using the `--remote-base-url` URL for remote linking
        Saving output to: target/doc/hello_world

        Run the following to see the results: 
        `mdbook serve target/doc/hello_world`
        (you will need to have mdbook installed)

        Or build html docs by running `scarb doc --build`
        "#});

    let content = std::fs::read_to_string(
        child_dir
            .path()
            .join("target/doc/hello_world/src/hello_world.md"),
    )
    .unwrap();

    let expected_link = format_expected_url(remote_base_url, "src/lib.cairo");
    assert!(content.contains(&expected_link));
}

#[test]
fn can_be_disabled() {
    let root_dir = TempDir::new().unwrap();
    root_dir.child(".gitignore");
    gitx::init(&root_dir);

    let child_dir = root_dir.child("hello_world");
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
              pub fn main() {
                println!("hellow")
              }
        "#})
        .manifest_package_extra(indoc! {r#"
            repository ="https://github.com/ExampleRepoOwner/ExampleRepoProject"
        "#})
        .build(&child_dir);
    gitx::commit(&root_dir);

    Scarb::quick_command()
        .arg("doc")
        .args(["--disable-remote-linking"])
        .current_dir(&child_dir)
        .assert()
        .success();

    // assert links do not exist
    for doc_file in ["hello_world.md", "hello_world-main.md"] {
        let content = std::fs::read_to_string(
            child_dir
                .path()
                .join("target/doc/hello_world/src/")
                .join(doc_file),
        )
        .unwrap();
        assert!(!content.contains("[source code] </a>"))
    }
}

#[test]
fn linking_enabled_no_url_provided() {
    let root_dir = TempDir::new().unwrap();
    root_dir.child(".gitignore");
    gitx::init(&root_dir);

    let child_dir = root_dir.child("hello_world");
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
              pub fn main() {
                println!("hellow")
              }
        "#})
        .build(&child_dir);

    Scarb::quick_command()
        .arg("doc")
        .current_dir(&child_dir)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: remote source linking is enabled, but no repository URL is configured,
            provide `--remote-base-url` or pass `--disable-remote-linking`,
            see https://docs.swmansion.com/scarb/docs/extensions/documentation-generation.html#linking-to-the-source-code-vcs-repository for details

        "#});
}

#[test]
fn json_output_forbids_remote_linking() {
    let root_dir = TempDir::new().unwrap();
    root_dir.child(".gitignore");
    gitx::init(&root_dir);

    let child_dir = root_dir.child("hello_world");
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
              pub fn main() {
                println!("hellow")
              }
        "#})
        .build(&child_dir);

    Scarb::quick_command()
        .arg("doc")
        .args([
            "--output-format",
            "json",
            "--remote-base-url",
            "https://github.com/ExampleRepoOwner/ExampleRepoProject",
        ])
        .current_dir(&child_dir)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: remote url linking is only supported for Markdown output format
            "#});
}

#[test]
fn warn_when_both_manifest_and_explicit_provided() {
    let root_dir = TempDir::new().unwrap();
    root_dir.child(".gitignore");
    gitx::init(&root_dir);

    let child_dir = root_dir.child("hello_world");
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
              pub fn main() {
                println!("hellow")
              }
        "#})
        .build(&child_dir);

    Scarb::quick_command()
        .arg("doc")
        .args([
            "--output-format",
            "json",
            "--remote-base-url",
            "https://github.com/ExampleRepoOwner/ExampleRepoProject",
        ])
        .current_dir(&child_dir)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: remote url linking is only supported for Markdown output format
            "#});
}

#[test]
fn git_discovery_failed() {
    let root_dir = TempDir::new().unwrap();
    root_dir.child(".gitignore");

    let child_dir = root_dir.child("hello_world");
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(indoc! {r#"
              pub fn main() {
                println!("hellow")
              }
        "#})
        .manifest_package_extra(indoc! {r#"
            repository ="https://github.com/ExampleRepoOwner/ExampleRepoProject"
        "#})
        .build(&child_dir);

    Scarb::quick_command()
        .arg("doc")
        .current_dir(&child_dir)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
        error: could not discover a Git repository, remote linking disabled
        "#});
}
