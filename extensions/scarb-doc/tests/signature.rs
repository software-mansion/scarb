use assert_fs::TempDir;
use scarb_test_support::{command::Scarb, project_builder::ProjectBuilder};

mod json_target;
use json_target::JsonTargetChecker;

const CONSTANT_SAMPLE_CODE: &str = include_str!("code/code_6_const.cairo");
const FUNCTION_SAMPLE_CODE: &str = include_str!("code/code_7_fn.cairo");
const TRAIT_SAMPLE_CODE: &str = include_str!("code/code_8_trait_impl.cairo");
const ALIAS_SAMPLE_CODE: &str = include_str!("code/code_9_alias.cairo");

#[test]
fn document_constant_signatures() {
    let root_dir = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(CONSTANT_SAMPLE_CODE)
        .build(&root_dir);

    Scarb::quick_snapbox()
        .arg("doc")
        .args(["--document-private-items", "--output-format", "json"])
        .current_dir(&root_dir)
        .assert()
        .success();

    JsonTargetChecker::default()
        .actual(&root_dir.path().join("target/doc/output.json"))
        .expected("./data/signature/json_const.json")
        .assert_files_match();
}

#[test]
fn document_function_signature() {
    let root_dir = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(FUNCTION_SAMPLE_CODE)
        .build(&root_dir);

    Scarb::quick_snapbox()
        .arg("doc")
        .args(["--document-private-items", "--output-format", "json"])
        .current_dir(&root_dir)
        .assert()
        .success();

    JsonTargetChecker::default()
        .actual(&root_dir.path().join("target/doc/output.json"))
        .expected("./data/signature/json_fn.json")
        .assert_files_match();
}

#[test]
fn document_trait_signature() {
    let root_dir = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(TRAIT_SAMPLE_CODE)
        .build(&root_dir);

    Scarb::quick_snapbox()
        .arg("doc")
        .args(["--document-private-items", "--output-format", "json"])
        .current_dir(&root_dir)
        .assert()
        .success();

    JsonTargetChecker::default()
        .actual(&root_dir.path().join("target/doc/output.json"))
        .expected("./data/signature/json_trait_impl.json")
        .assert_files_match();
}

#[test]
fn document_alias_signature() {
    let root_dir = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .lib_cairo(ALIAS_SAMPLE_CODE)
        .build(&root_dir);

    Scarb::quick_snapbox()
        .arg("doc")
        .args(["--document-private-items", "--output-format", "json"])
        .current_dir(&root_dir)
        .assert()
        .success();

    JsonTargetChecker::default()
        .actual(&root_dir.path().join("target/doc/output.json"))
        .expected("./data/signature/json_alias.json")
        .assert_files_match();
}
