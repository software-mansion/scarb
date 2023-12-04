use assert_fs::prelude::*;
use assert_fs::TempDir;

use scarb_test_support::fsx::ChildPathEx;

use indoc::indoc;

use scarb_test_support::command::Scarb;

use scarb_test_support::project_builder::ProjectBuilder;
use serde_json::Value;

const SIMPLE_TEST: &str = indoc! {r#"
    #[cfg(test)]
    mod tests {
        #[test]
        fn test() {
            assert(true == true, 'it works!')
        }
    }
    "#
};

#[test]
fn forge_test_locations() {
    let t = TempDir::new().unwrap();
    let pkg1 = t.child("forge");

    ProjectBuilder::start()
        .name("forge_test")
        .lib_cairo(SIMPLE_TEST)
        .src("tests/lib.cairo", SIMPLE_TEST)
        .build(&pkg1);
    Scarb::quick_snapbox()
        .arg("snforge-test-collector")
        .current_dir(&pkg1)
        .assert()
        .success();

    let snforge_sierra = pkg1
        .child("target/dev/snforge/forge_test.snforge_sierra.json")
        .read_to_string();

    let json: Value = serde_json::from_str(&snforge_sierra).unwrap();

    assert_eq!(&json[0]["test_cases"][0]["name"], "forge_test::tests::test");
    assert_eq!(&json[1]["test_cases"][0]["name"], "tests::tests::test");

    assert_eq!(&json[0]["test_cases"][0]["available_gas"], &Value::Null);
    assert_eq!(&json[0]["test_cases"][0]["expected_result"], "Success");
    assert_eq!(&json[0]["test_cases"][0]["fork_config"], &Value::Null);
    assert_eq!(&json[0]["test_cases"][0]["fuzzer_config"], &Value::Null);
    assert_eq!(&json[0]["test_cases"][0]["ignored"], false);
}
#[test]
fn forge_test_wrong_location() {
    let t = TempDir::new().unwrap();
    let pkg1 = t.child("forge");

    ProjectBuilder::start()
        .name("forge_test")
        .src("a/lib.cairo", SIMPLE_TEST)
        .build(&pkg1);
    Scarb::quick_snapbox()
        .arg("snforge-test-collector")
        .current_dir(&pkg1)
        .assert()
        .success();

    let snforge_sierra = pkg1
        .child("target/dev/snforge/forge_test.snforge_sierra.json")
        .read_to_string();

    let json: Value = serde_json::from_str(&snforge_sierra).unwrap();
    assert_eq!(&json[0]["test_cases"][0], &Value::Null);

}

const WITH_MANY_ATTRIBUTES_TEST: &str = indoc! {r#"
    #[cfg(test)]
    mod tests {
        #[ignore]
        #[fork(url: "http://your.rpc.url", block_id: BlockId::Number(123))]
        #[should_panic]
        #[fuzzer(runs: 22, seed: 38)]
        #[test]
        fn test(a: felt252) {
            assert(true == true, 'it works!')
        }
    }
    "#
};

#[test]
fn forge_test_with_attributes() {
    let t = TempDir::new().unwrap();
    let pkg1 = t.child("forge");

    ProjectBuilder::start()
        .name("forge_test")
        .lib_cairo(WITH_MANY_ATTRIBUTES_TEST)
        .build(&pkg1);
    Scarb::quick_snapbox()
        .arg("snforge-test-collector")
        .current_dir(&pkg1)
        .assert()
        .success();

    let snforge_sierra = pkg1
        .child("target/dev/snforge/forge_test.snforge_sierra.json")
        .read_to_string();

    let json: Value = serde_json::from_str(&snforge_sierra).unwrap();

    assert_eq!(&json[0]["test_cases"][0]["available_gas"], &Value::Null);
    assert_eq!(
        &json[0]["test_cases"][0]["expected_result"]["Panics"],
        "Any"
    );
    assert_eq!(
        &json[0]["test_cases"][0]["fork_config"]["Params"]["block_id_type"],
        "Number"
    );
    assert_eq!(
        &json[0]["test_cases"][0]["fork_config"]["Params"]["block_id_value"],
        "123"
    );
    assert_eq!(
        &json[0]["test_cases"][0]["fork_config"]["Params"]["url"],
        "http://your.rpc.url"
    );
    assert_eq!(
        &json[0]["test_cases"][0]["fuzzer_config"]["fuzzer_runs"],
        22
    );
    assert_eq!(
        &json[0]["test_cases"][0]["fuzzer_config"]["fuzzer_seed"],
        38
    );
    assert_eq!(&json[0]["test_cases"][0]["ignored"], true);
    assert_eq!(&json[0]["test_cases"][0]["name"], "forge_test::tests::test");
}

const FORK_TAG_TEST: &str = indoc! {r#"
    #[cfg(test)]
    mod tests {
        #[fork(url: "http://your.rpc.url", block_id: BlockId::Tag(Latest))]
        #[test]
        fn test() {
            assert(true == true, 'it works!')
        }
    }
    "#
};

#[test]
fn forge_test_with_fork_tag_attribute() {
    let t = TempDir::new().unwrap();
    let pkg1 = t.child("forge");

    ProjectBuilder::start()
        .name("forge_test")
        .lib_cairo(FORK_TAG_TEST)
        .build(&pkg1);
    Scarb::quick_snapbox()
        .arg("snforge-test-collector")
        .current_dir(&pkg1)
        .assert()
        .success();

    let snforge_sierra = pkg1
        .child("target/dev/snforge/forge_test.snforge_sierra.json")
        .read_to_string();

    let json: Value = serde_json::from_str(&snforge_sierra).unwrap();

    assert_eq!(
        &json[0]["test_cases"][0]["fork_config"]["Params"]["block_id_type"],
        "Tag"
    );
    assert_eq!(
        &json[0]["test_cases"][0]["fork_config"]["Params"]["block_id_value"],
        "Latest"
    );
    assert_eq!(
        &json[0]["test_cases"][0]["fork_config"]["Params"]["url"],
        "http://your.rpc.url"
    );

    assert_eq!(&json[0]["test_cases"][0]["name"], "forge_test::tests::test");
}

const FORK_HASH_TEST: &str = indoc! {r#"
    #[cfg(test)]
    mod tests {
        #[fork(url: "http://your.rpc.url", block_id: BlockId::Hash(123))]
        #[test]
        fn test() {
            assert(true == true, 'it works!')
        }
    }
    "#
};

#[test]
fn forge_test_with_fork_hash_attribute() {
    let t = TempDir::new().unwrap();
    let pkg1 = t.child("forge");

    ProjectBuilder::start()
        .name("forge_test")
        .lib_cairo(FORK_HASH_TEST)
        .build(&pkg1);
    Scarb::quick_snapbox()
        .arg("snforge-test-collector")
        .current_dir(&pkg1)
        .assert()
        .success();

    let snforge_sierra = pkg1
        .child("target/dev/snforge/forge_test.snforge_sierra.json")
        .read_to_string();

    let json: Value = serde_json::from_str(&snforge_sierra).unwrap();

    assert_eq!(
        &json[0]["test_cases"][0]["fork_config"]["Params"]["block_id_type"],
        "Hash"
    );
    assert_eq!(
        &json[0]["test_cases"][0]["fork_config"]["Params"]["block_id_value"],
        "123"
    );
    assert_eq!(
        &json[0]["test_cases"][0]["fork_config"]["Params"]["url"],
        "http://your.rpc.url"
    );

    assert_eq!(&json[0]["test_cases"][0]["name"], "forge_test::tests::test");
}

const SHOULD_PANIC_TEST: &str = indoc! {r#"
    #[cfg(test)]
    mod tests {
        #[should_panic(expected: ('panic message', 'eventual second message',))]
        #[test]
        fn test() {
            assert(true == true, 'it works!')
        }
    }
    "#
};

#[test]
fn forge_test_with_should_panic_message_attribute() {
    let t = TempDir::new().unwrap();
    let pkg1 = t.child("forge");

    ProjectBuilder::start()
        .name("forge_test")
        .lib_cairo(SHOULD_PANIC_TEST)
        .build(&pkg1);
    Scarb::quick_snapbox()
        .arg("snforge-test-collector")
        .current_dir(&pkg1)
        .assert()
        .success();

    let snforge_sierra = pkg1
        .child("target/dev/snforge/forge_test.snforge_sierra.json")
        .read_to_string();

    let json: Value = serde_json::from_str(&snforge_sierra).unwrap();
    dbg!();
    assert_eq!(
        &json[0]["test_cases"][0]["expected_result"]["Panics"].to_string(),
        "{\"Exact\":[{\"value\":{\"val\":[1935763301,544040307,1634625891,112]}},{\"value\":{\"val\":[1935763301,544040307,1668247140,1814066021,1853125985,6649445]}}]}"
    );

    assert_eq!(&json[0]["test_cases"][0]["name"], "forge_test::tests::test");
}
