use assert_fs::TempDir;
use assert_fs::prelude::PathChild;
use camino::Utf8PathBuf;
use scarb_test_support::command::Scarb;
use scarb_test_support::expect_dir::ExpectDir;
use snapbox::Data;

const HELLO_WORLD_PATH: &str = "tests/data/hello_world";

#[test]
fn build_hello_world() {
    let t = TempDir::new().unwrap();
    let output = t.child("output");
    let output_path = output.path().display().to_string();
    let example = Utf8PathBuf::from(HELLO_WORLD_PATH);
    let input_path = example.join("input");
    let expected_output_path = example.join("output");
    Scarb::quick_command()
        .arg("mdbook")
        .arg(format!("--input={input_path}"))
        .arg(format!("--output={output_path}",))
        .assert()
        .success()
        .stdout_eq(Data::from("").raw());
    ExpectDir::lenient()
        .expected(expected_output_path.as_ref())
        .actual(output_path.as_ref())
        .assert_all_files_match();
}
