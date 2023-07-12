use assert_fs::fixture::ChildPath;
use std::fs;

use assert_fs::prelude::*;
use cairo_lang_starknet::contract_class::ContractClass;
use indoc::{formatdoc, indoc};
use itertools::Itertools;

use scarb_test_support::command::Scarb;
use scarb_test_support::contracts::{BALANCE_CONTRACT, FORTY_TWO_CONTRACT, HELLO_CONTRACT};
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::project_builder::ProjectBuilder;

fn compile_dep_test_case(hello: &ChildPath, world: &ChildPath, target_extra: &str) {
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [lib]
            [[target.starknet-contract]]
        "#})
        .dep_starknet()
        .lib_cairo(format!("{}\n{}", BALANCE_CONTRACT, HELLO_CONTRACT))
        .build(hello);

    ProjectBuilder::start()
        .name("world")
        .version("0.1.0")
        .dep("hello", r#" path = "../hello" "#)
        .manifest_extra(formatdoc! {r#"
            [[target.starknet-contract]]
            {target_extra}
        "#})
        .dep_starknet()
        .lib_cairo(format!("{}\n{}", FORTY_TWO_CONTRACT, HELLO_CONTRACT))
        .build(world);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(world)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..] Compiling world v0.1.0 ([..]/Scarb.toml)
            [..]  Finished release target(s) in [..] seconds
        "#});
}

#[test]
fn compile_imported_contracts() {
    let t = assert_fs::TempDir::new().unwrap();
    let hello = t.child("hello");
    let world = t.child("world");
    compile_dep_test_case(
        &hello,
        &world,
        indoc! {r#"
        build-external-contracts = [
            "hello::Balance",
        ]
    "#},
    );

    assert_eq!(
        world.child("target/dev").files(),
        vec![
            "world.starknet_artifacts.json",
            "world_Balance.sierra.json",
            "world_FortyTwo.sierra.json",
            "world_HelloContract.sierra.json",
        ]
    );
    world
        .child("target/dev/world_Balance.sierra.json")
        .assert_is_json::<ContractClass>();
    world
        .child("target/dev/world_FortyTwo.sierra.json")
        .assert_is_json::<ContractClass>();
    world
        .child("target/dev/world_HelloContract.sierra.json")
        .assert_is_json::<ContractClass>();
}

#[test]
fn compile_multiple_imported_contracts() {
    let t = assert_fs::TempDir::new().unwrap();
    let hello = t.child("hello");
    let world = t.child("world");
    compile_dep_test_case(
        &hello,
        &world,
        indoc! {r#"
        build-external-contracts = [
            "hello::Balance",
            "hello::HelloContract",
        ]
    "#},
    );

    assert_eq!(
        world.child("target/dev").files(),
        vec![
            "world.starknet_artifacts.json",
            "world_Balance.sierra.json",
            "world_FortyTwo.sierra.json",
            "world_hello_HelloContract.sierra.json",
            "world_world_HelloContract.sierra.json",
        ]
    );
    world
        .child("target/dev/world_Balance.sierra.json")
        .assert_is_json::<ContractClass>();
    world
        .child("target/dev/world_hello_HelloContract.sierra.json")
        .assert_is_json::<ContractClass>();
    world
        .child("target/dev/world_FortyTwo.sierra.json")
        .assert_is_json::<ContractClass>();
    world
        .child("target/dev/world_hello_HelloContract.sierra.json")
        .assert_is_json::<ContractClass>();

    // Check starknet artifacts content
    let starknet_artifacts = world.child("target/dev/world.starknet_artifacts.json");
    starknet_artifacts.assert_is_json::<serde_json::Value>();
    let content = fs::read_to_string(&starknet_artifacts).unwrap();
    let json: serde_json::Value = serde_json::from_str(content.as_ref()).unwrap();
    let contracts = json
        .as_object()
        .unwrap()
        .get("contracts")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(contracts.len(), 4);
    assert_eq!(
        contracts
            .iter()
            .map(|c| {
                let c = c.as_object().unwrap();
                let pkg = c.get("package_name").unwrap().as_str().unwrap();
                let name = c.get("contract_name").unwrap().as_str().unwrap();
                let sierra = c
                    .get("artifacts")
                    .unwrap()
                    .as_object()
                    .unwrap()
                    .get("sierra")
                    .unwrap()
                    .as_str()
                    .unwrap();
                (pkg, name, sierra)
            })
            .sorted()
            .collect::<Vec<_>>(),
        vec![
            ("hello", "Balance", "world_Balance.sierra.json"),
            (
                "hello",
                "HelloContract",
                "world_hello_HelloContract.sierra.json"
            ),
            ("world", "FortyTwo", "world_FortyTwo.sierra.json"),
            (
                "world",
                "HelloContract",
                "world_world_HelloContract.sierra.json"
            ),
        ]
    );
}

#[test]
fn build_external_full_path() {
    let t = assert_fs::TempDir::new().unwrap();
    let hello = t.child("hello");
    let world = t.child("world");

    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [lib]
            [[target.starknet-contract]]
        "#})
        .dep_starknet()
        .lib_cairo(indoc! {r#"
            mod lorem;
        "#})
        .src(
            "src/lorem.cairo",
            indoc! {r#"
            mod ipsum;
        "#},
        )
        .src(
            "src/lorem/ipsum.cairo",
            format!("{}\n{}", BALANCE_CONTRACT, HELLO_CONTRACT),
        )
        .build(&hello);

    ProjectBuilder::start()
        .name("world")
        .version("0.1.0")
        .dep("hello", r#" path = "../hello" "#)
        .manifest_extra(formatdoc! {r#"
            [[target.starknet-contract]]
            build-external-contracts = [
                "hello::lorem::ipsum::Balance",
                "hello::lorem::ipsum::HelloContract",
            ]
        "#})
        .dep_starknet()
        .lib_cairo(format!("{}\n{}", FORTY_TWO_CONTRACT, HELLO_CONTRACT))
        .build(&world);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&world)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..] Compiling world v0.1.0 ([..]/Scarb.toml)
            [..]  Finished release target(s) in [..] seconds
        "#});
    assert_eq!(
        world.child("target/dev").files(),
        vec![
            "world.starknet_artifacts.json",
            "world_Balance.sierra.json",
            "world_FortyTwo.sierra.json",
            "world_hello_lorem_ipsum_HelloContract.sierra.json",
            "world_world_HelloContract.sierra.json",
        ]
    );
}
