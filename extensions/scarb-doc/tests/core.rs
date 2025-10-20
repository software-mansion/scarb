use assert_fs::TempDir;
use indoc::indoc;
use scarb_metadata::Metadata;
use scarb_test_support::command::{CommandExt, Scarb};
use scarb_test_support::fsx;
use scarb_test_support::project_builder::ProjectBuilder;
use std::path::PathBuf;

#[test]
fn can_doc_corelib() {
    let t = TempDir::new().unwrap();
    // Find path to corelib.
    ProjectBuilder::start().name("hello").build(&t);
    let metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version", "1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();
    let core = metadata.packages.iter().find(|p| p.name == "core").unwrap();
    let core = core.root.clone();
    // Doc corelib.
    Scarb::quick_snapbox()
        .arg("doc")
        .arg("--build")
        .current_dir(core)
        .assert()
        .success();
}

#[test]
fn stdout_output_info() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello_world").build(&t);

    Scarb::quick_snapbox()
        .arg("doc")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            Saving output to: target/doc/hello_world

            Run the following to see the results: 
            `mdbook serve target/doc/hello_world`
            (you will need to have mdbook installed)
            
            Or build html docs by running `scarb doc --build`
        "#});

    Scarb::quick_snapbox()
        .arg("doc")
        .args(["--output-format", "json"])
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            Saving output to: target/doc/output.json
        "#});

    let expected_path = t.join(PathBuf::from("target/doc/hello_world/book/index.html"));

    Scarb::quick_snapbox()
        .arg("doc")
        .args(["--build"])
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(format!(
            indoc! {r#"
        Saving output to: target/doc/hello_world
        Saving build output to: target/doc/hello_world/book
        
        Run the following to see the results: 
        `mdbook serve target/doc/hello_world`

        Or open the following in your browser: 
        `{}`
    "#},
            fsx::canonicalize(&expected_path)
                .unwrap()
                .to_string_lossy()
                .replace("\\", "/"),
        ));
}
