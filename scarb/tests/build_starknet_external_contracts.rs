use assert_fs::fixture::ChildPath;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use cairo_lang_starknet_classes::contract_class::ContractClass;
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
        .dep("hello", hello)
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
            [..]  Finished release target(s) in [..]
        "#});
}

#[test]
fn compile_imported_contracts() {
    let t = TempDir::new().unwrap();
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
            "world_Balance.contract_class.json",
            "world_FortyTwo.contract_class.json",
            "world_HelloContract.contract_class.json",
        ]
    );
    world
        .child("target/dev/world_Balance.contract_class.json")
        .assert_is_json::<ContractClass>();
    world
        .child("target/dev/world_FortyTwo.contract_class.json")
        .assert_is_json::<ContractClass>();
    world
        .child("target/dev/world_HelloContract.contract_class.json")
        .assert_is_json::<ContractClass>();
}

#[test]
fn compile_multiple_imported_contracts() {
    let t = TempDir::new().unwrap();
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
            "world_Balance.contract_class.json",
            "world_FortyTwo.contract_class.json",
            "world_hello_HelloContract.contract_class.json",
            "world_world_HelloContract.contract_class.json",
        ]
    );
    world
        .child("target/dev/world_Balance.contract_class.json")
        .assert_is_json::<ContractClass>();
    world
        .child("target/dev/world_hello_HelloContract.contract_class.json")
        .assert_is_json::<ContractClass>();
    world
        .child("target/dev/world_FortyTwo.contract_class.json")
        .assert_is_json::<ContractClass>();
    world
        .child("target/dev/world_hello_HelloContract.contract_class.json")
        .assert_is_json::<ContractClass>();

    // Check starknet artifacts content
    let starknet_artifacts = world.child("target/dev/world.starknet_artifacts.json");
    starknet_artifacts.assert_is_json::<serde_json::Value>();
    let content = starknet_artifacts.read_to_string();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
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
            ("hello", "Balance", "world_Balance.contract_class.json"),
            (
                "hello",
                "HelloContract",
                "world_hello_HelloContract.contract_class.json"
            ),
            ("world", "FortyTwo", "world_FortyTwo.contract_class.json"),
            (
                "world",
                "HelloContract",
                "world_world_HelloContract.contract_class.json"
            ),
        ]
    );
}

#[test]
fn build_external_full_path() {
    let t = TempDir::new().unwrap();
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
        .dep("hello", &hello)
        .manifest_extra(indoc! {r#"
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
            [..]  Finished release target(s) in [..]
        "#});
    assert_eq!(
        world.child("target/dev").files(),
        vec![
            "world.starknet_artifacts.json",
            "world_Balance.contract_class.json",
            "world_FortyTwo.contract_class.json",
            "world_hello_lorem_ipsum_HelloContract.contract_class.json",
            "world_world_HelloContract.contract_class.json",
        ]
    );
}

#[test]
fn compile_multiple_with_glob_path() {
    let t = TempDir::new().unwrap();
    let hello = t.child("hello");
    let world = t.child("world");
    compile_dep_test_case(
        &hello,
        &world,
        indoc! {r#"
            build-external-contracts = [
                "hello::*",
            ]
        "#},
    );

    assert_eq!(
        world.child("target/dev").files(),
        vec![
            "world.starknet_artifacts.json",
            "world_Balance.contract_class.json",
            "world_FortyTwo.contract_class.json",
            "world_hello_HelloContract.contract_class.json",
            "world_world_HelloContract.contract_class.json"
        ]
    );
    world
        .child("target/dev/world_Balance.contract_class.json")
        .assert_is_json::<ContractClass>();
    world
        .child("target/dev/world_hello_HelloContract.contract_class.json")
        .assert_is_json::<ContractClass>();
    world
        .child("target/dev/world_FortyTwo.contract_class.json")
        .assert_is_json::<ContractClass>();
    world
        .child("target/dev/world_world_HelloContract.contract_class.json")
        .assert_is_json::<ContractClass>();

    // Check starknet artifacts content
    let starknet_artifacts = world.child("target/dev/world.starknet_artifacts.json");
    starknet_artifacts.assert_is_json::<serde_json::Value>();
    let content = starknet_artifacts.read_to_string();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
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
            ("hello", "Balance", "world_Balance.contract_class.json"),
            (
                "hello",
                "HelloContract",
                "world_hello_HelloContract.contract_class.json"
            ),
            ("world", "FortyTwo", "world_FortyTwo.contract_class.json"),
            (
                "world",
                "HelloContract",
                "world_world_HelloContract.contract_class.json"
            )
        ]
    );
}

#[test]
fn compile_multiple_with_glob_subpath() {
    let t = TempDir::new().unwrap();
    let x = t.child("x");
    let y = t.child("y");

    ProjectBuilder::start()
        .name("y")
        .version("1.0.0")
        .dep_starknet()
        .lib_cairo(r#"mod subfolder;"#)
        .src("src/subfolder.cairo", r#"mod b; mod c;"#)
        .src(
            "src/subfolder/b.cairo",
            indoc! {r#"
            #[starknet::contract]
            mod B {
                #[storage]
                struct Storage {}
            }
        "#},
        )
        .src(
            "src/subfolder/c.cairo",
            indoc! {r#"
            #[starknet::contract]
            mod C {
                #[storage]
                struct Storage {}
            }
        "#},
        )
        .build(&y);

    ProjectBuilder::start()
        .name("x")
        .version("1.0.0")
        .dep_starknet()
        .dep("y", &y)
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            build-external-contracts = ["y::subfolder::*"]
        "#})
        .lib_cairo(indoc! {r#"
            #[starknet::contract]
            mod A {
                use y::subfolder::b::B;
                use y::subfolder::c::C;

                #[storage]
                struct Storage {}
            }
        "#})
        .build(&x);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&x)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..] Compiling x v1.0.0 ([..]Scarb.toml)
            [..]  Finished release target(s) in [..]
        "#});
}

#[test]
fn compile_with_bad_glob_path() {
    let t = TempDir::new().unwrap();
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
        .lib_cairo(format!("{}\n{}", BALANCE_CONTRACT, HELLO_CONTRACT))
        .build(&hello);

    ProjectBuilder::start()
        .name("world")
        .version("0.1.0")
        .dep("hello", &hello)
        .manifest_extra(formatdoc! {r#"
            [[target.starknet-contract]]
            build-external-contracts = ["hello::**",]
        "#})
        .dep_starknet()
        .lib_cairo(format!("{}\n{}", FORTY_TWO_CONTRACT, HELLO_CONTRACT))
        .build(&world);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(t.child("world"))
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Compiling world v0.1.0 ([..]/Scarb.toml)
        error: external contract path `hello::**` has multiple global path selectors, only one '*' selector is allowed
        error: could not compile `world` due to previous error
        "#});
}

#[test]
fn will_warn_about_unmatched_paths() {
    let t = TempDir::new().unwrap();
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
        .dep("hello", &hello)
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            build-external-contracts = [
                "hello::lorem::ipsum::Balance",
                "hello::lorem::ipsum::HelloContract",
                "hello::lorem::mopsum::*",
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
            warn: external contracts not found for selectors: `hello::lorem::mopsum::*`
            [..]  Finished release target(s) in [..]
        "#});
    assert_eq!(
        world.child("target/dev").files(),
        vec![
            "world.starknet_artifacts.json",
            "world_Balance.contract_class.json",
            "world_FortyTwo.contract_class.json",
            "world_hello_lorem_ipsum_HelloContract.contract_class.json",
            "world_world_HelloContract.contract_class.json",
        ]
    );
}
