use assert_fs::TempDir;
use indoc::indoc;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn build_script_runs_before_compilation() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
            [scripts]
            build = "echo 'Prebuild script executed'"
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Running `build` script for `hello`
            Prebuild script executed
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
        "#});
}
