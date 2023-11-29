use assert_fs::prelude::*;
use assert_fs::TempDir;

use scarb_test_support::fsx::ChildPathEx;

use indoc::indoc;

use scarb_test_support::command::Scarb;

use scarb_test_support::project_builder::ProjectBuilder;
use serde_json::Value;

const SIMPLE_TEST: &str = indoc! {r#"
    #[test]
    fn test() {
        assert(true == true, 'it works!')
    }
    "#
};

#[test]
fn forge_test() {
    let t = TempDir::new().unwrap();
    let pkg1 = t.child("forge");

    ProjectBuilder::start()
        .name("forge_test")
        .lib_cairo(SIMPLE_TEST)
        .build(&pkg1);
    Scarb::quick_snapbox()
        .arg("snforge-test-collector")
        .current_dir(&pkg1)
        .assert()
        .success();

    let a = pkg1
        .child("target/dev/snforge/forge_test.snforge_sierra.json")
        .read_to_string();
    let json: Value = serde_json::from_str(&a).unwrap();

    assert_eq!(&json[0]["test_cases"][0]["name"], "forge_test::test");
}
