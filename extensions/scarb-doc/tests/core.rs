use assert_fs::TempDir;
use indoc::formatdoc;
use scarb_doc::metadata::get_target_dir;
use scarb_metadata::Metadata;
use scarb_test_support::command::{CommandExt, Scarb};
use scarb_test_support::project_builder::ProjectBuilder;

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
        .current_dir(core)
        .assert()
        .success();
}

#[test]
fn stdout_output_info() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello_world").build(&t);

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version", "1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    Scarb::quick_snapbox()
        .arg("doc")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(formatdoc! {r#"
                Saved to directory: {dir}/doc
                "#,
                dir = get_target_dir(&metadata).as_str()
        });
}
