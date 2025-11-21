use assert_fs::TempDir;
use assert_fs::prelude::PathChild;
use indoc::formatdoc;
use scarb_test_support::command::Scarb;
use scarb_test_support::contracts::HELLO_CONTRACT;
use scarb_test_support::project_builder::ProjectBuilder;
use std::fs;
use test_case::test_case;

#[test_case("lib", "sierra", ".sierra.json")]
#[test_case("lib", "sierra-text", ".sierra")]
#[test_case("lib", "casm", ".casm")]
#[test_case("[target.starknet-contract]", "sierra", ".starknet_artifacts.json")]
#[test_case(
    "[target.starknet-contract]",
    "sierra",
    "_HelloContract.contract_class.json"
)]
#[test_case(
    "[target.starknet-contract]",
    "casm",
    "_HelloContract.compiled_contract_class.json"
)]
#[test_case("test", "", "_unittest.test.json")]
#[test_case("test", "", "_unittest.test.sierra.json")]
#[test_case("test", "", "_unittest.test.starknet_artifacts.json")]
#[test_case("test", "", "_unittest_HelloContract.test.contract_class.json")]
#[test_case("test", "", "_integrationtest.test.json")]
#[test_case("test", "", "_integrationtest.test.sierra.json")]
#[test_case("test", "", "_integrationtest.test.starknet_artifacts.json")]
#[test_case("test", "", "_integrationtest_HelloContract.test.contract_class.json")]
#[test_case("executable", "", ".executable.json")]
#[test_case("executable", "sierra", ".executable.sierra.json")]
fn changed_artifact(target: &str, prop: &str, file_suffix: &str) {
    // We affix cache dir location, as the corelib path is part of the fingerprint.
    let cache_dir = TempDir::new().unwrap().child("c");
    let t = TempDir::new().unwrap();

    let prop = if prop.is_empty() {
        String::new()
    } else {
        format!("{prop} = true")
    };
    let target_section = if target != "test" {
        formatdoc! {r#"
            [{}]
            {}
        "#, target, prop}
    } else {
        String::new()
    };
    let profile_section = if target == "executable" {
        "[cairo]\nenable-gas = false".to_string()
    } else {
        String::new()
    };

    ProjectBuilder::start()
        .name("hello")
        .dep_starknet()
        .dep_cairo_execute()
        .manifest_extra(format!("{target_section}\n{profile_section}"))
        .lib_cairo(formatdoc! {r#"
            {HELLO_CONTRACT}

            #[executable]
            fn main() -> felt252 {{
                12
            }}
        "#})
        .src("tests/some.cairo", "")
        .build(&t);

    let run_scarb = || {
        let mut cmd = Scarb::new().cache(cache_dir.path()).command().arg("build");
        if target == "test" {
            cmd = cmd.arg("--test");
        }
        cmd.current_dir(&t).assert().success();
    };

    run_scarb();

    let artifact_path = t.child(format!("target/dev/hello{file_suffix}"));
    let get_artifact_content = || fs::read_to_string(&artifact_path).unwrap();

    let old_artifact_content = get_artifact_content();

    // change contents of the artifact file
    fs::write(&artifact_path, "changed").unwrap();
    let changed_artifact_content = get_artifact_content();
    assert_ne!(old_artifact_content, changed_artifact_content);

    run_scarb();

    assert_ne!(changed_artifact_content, get_artifact_content());
}

#[test]
fn removed_artifact() {
    // We affix cache dir location, as the corelib path is part of the fingerprint.
    let cache_dir = TempDir::new().unwrap().child("c");
    let t = TempDir::new().unwrap();

    ProjectBuilder::start().name("hello").build(&t);

    let run_scarb = || {
        Scarb::new()
            .cache(cache_dir.path())
            .command()
            .arg("build")
            .current_dir(&t)
            .assert()
            .success();
    };

    run_scarb();

    let artifact_path = t.child("target/dev/hello.sierra.json");
    assert!(artifact_path.exists());

    // remove the artifact file
    fs::remove_file(&artifact_path).unwrap();
    assert!(!artifact_path.exists());

    run_scarb();

    assert!(artifact_path.exists());
}
