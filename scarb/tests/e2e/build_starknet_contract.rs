use std::fs;
use std::fs::File;
use std::io::BufReader;

use assert_fs::fixture::ChildPath;
use assert_fs::prelude::*;
use cairo_lang_starknet::casm_contract_class::CasmContractClass;
use cairo_lang_starknet::contract_class::ContractClass;
use indoc::{formatdoc, indoc};
use itertools::Itertools;
use once_cell::sync::Lazy;
use predicates::prelude::*;
use serde::de::DeserializeOwned;

use crate::support::command::Scarb;
use crate::support::fsx::ChildPathEx;
use crate::support::project_builder::ProjectBuilder;

const BALANCE_CONTRACT_INTERFACE: &str = indoc! {r#"
    #[starknet::interface]
    trait IBalance<T> {
        // Returns the current balance.
        fn get(self: @T) -> u128;
        // Increases the balance by the given amount.
        fn increase(ref self: T, a: u128);
    }
"#};

const BALANCE_CONTRACT_IMPLEMENTATION: &str = indoc! {r#"
    #[starknet::contract]
    mod Balance {
        use traits::Into;

        #[storage]
        struct Storage {
            value: u128,
        }

        #[constructor]
        fn constructor(ref self: ContractState, value_: u128) {
            self.value.write(value_);
        }

        #[external(v0)]
        impl Balance of super::IBalance<ContractState> {
            fn get(self: @ContractState) -> u128 {
                self.value.read()
            }
            fn increase(ref self: ContractState, a: u128)  {
                self.value.write( self.value.read() + a );
            }
        }
    }
"#};

static BALANCE_CONTRACT: Lazy<String> =
    Lazy::new(|| format!("{BALANCE_CONTRACT_INTERFACE}\n{BALANCE_CONTRACT_IMPLEMENTATION}"));

const FORTY_TWO_CONTRACT: &str = indoc! {r#"
    #[starknet::interface]
    trait IFortyTwo<T> {
        fn answer(ref self: T) -> felt252;
    }
    #[starknet::contract]
    mod FortyTwo {
        #[storage]
        struct Storage {}
        #[external(v0)]
        fn answer(ref self: ContractState) -> felt252 { 42 }
        impl FortyTwo of super::IFortyTwo<ContractState> {
            fn answer(ref self: ContractState) -> felt252 { 42 }
        }
    }
"#};

const HELLO_CONTRACT: &str = indoc! {r#"
    #[starknet::interface]
    trait IHelloContract<T> {
        fn answer(ref self: T) -> felt252;
    }
    #[starknet::contract]
    mod HelloContract {
        #[storage]
        struct Storage {}
        #[external(v0)]
        impl HelloContract of super::IHelloContract<ContractState> {
            fn answer(ref self: ContractState) -> felt252 { 'hello' }
        }
    }
"#};

fn assert_is_json<T: DeserializeOwned>(child: &ChildPath) -> T {
    let file = File::open(child.path()).unwrap();
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).unwrap()
}

#[test]
fn compile_starknet_contract() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra("[[target.starknet-contract]]")
        .dep_starknet()
        .lib_cairo(BALANCE_CONTRACT.as_str())
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        [..]  Finished release target(s) in [..]
        "#});

    assert_eq!(
        t.child("target/dev").files(),
        vec!["hello.starknet_artifacts.json", "hello_Balance.sierra.json"]
    );

    assert_is_json::<ContractClass>(&t.child("target/dev/hello_Balance.sierra.json"));
}

#[test]
fn compile_starknet_contract_to_casm() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            sierra = false
            casm = true
        "#})
        .dep_starknet()
        .lib_cairo(BALANCE_CONTRACT.as_str())
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        [..]  Finished release target(s) in [..]
        "#});

    assert_eq!(
        t.child("target/dev").files(),
        vec!["hello.starknet_artifacts.json", "hello_Balance.casm.json"]
    );

    assert_is_json::<CasmContractClass>(&t.child("target/dev/hello_Balance.casm.json"));
}

#[test]
fn compile_many_contracts() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [lib]
            sierra = true
            casm = true

            [[target.starknet-contract]]
            name = "a"

            [[target.starknet-contract]]
            name = "b"
        "#})
        .dep_starknet()
        .lib_cairo(indoc! {r#"
            mod balance;
            mod forty_two;
        "#})
        .src("src/balance.cairo", BALANCE_CONTRACT.as_str())
        .src("src/forty_two.cairo", FORTY_TWO_CONTRACT)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Compiling lib(hello) hello v0.1.0 ([..])
        [..] Compiling starknet-contract(a) hello v0.1.0 ([..])
        [..] Compiling starknet-contract(b) hello v0.1.0 ([..])
        [..]  Finished release target(s) in [..]
        "#});

    assert_eq!(
        t.child("target/dev").files(),
        vec![
            "a.starknet_artifacts.json",
            "a_Balance.sierra.json",
            "a_FortyTwo.sierra.json",
            "b.starknet_artifacts.json",
            "b_Balance.sierra.json",
            "b_FortyTwo.sierra.json",
            "hello.casm",
            "hello.sierra",
        ]
    );

    for json in [
        "a_Balance.sierra.json",
        "a_FortyTwo.sierra.json",
        "b_Balance.sierra.json",
        "b_FortyTwo.sierra.json",
    ] {
        assert_is_json::<ContractClass>(&t.child("target/dev").child(json));
    }

    assert_is_json::<serde_json::Value>(&t.child("target/dev/a.starknet_artifacts.json"));
    assert_is_json::<serde_json::Value>(&t.child("target/dev/b.starknet_artifacts.json"));
}

#[test]
fn compile_same_name_contracts() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
        "#})
        .dep_starknet()
        .lib_cairo(indoc! {r#"
            mod forty_two;
            mod world;
        "#})
        .src("src/forty_two.cairo", FORTY_TWO_CONTRACT)
        .src("src/world.cairo", FORTY_TWO_CONTRACT)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        [..]  Finished release target(s) in [..]
        "#});

    assert_eq!(
        t.child("target/dev").files(),
        vec![
            "hello.starknet_artifacts.json",
            "hello_hello_forty_two_FortyTwo.sierra.json",
            "hello_hello_world_FortyTwo.sierra.json",
        ]
    );

    assert_is_json::<serde_json::Value>(&t.child("target/dev/hello.starknet_artifacts.json"));
    assert_is_json::<serde_json::Value>(
        &t.child("target/dev/hello_hello_forty_two_FortyTwo.sierra.json"),
    );
    assert_is_json::<serde_json::Value>(
        &t.child("target/dev/hello_hello_world_FortyTwo.sierra.json"),
    );
}

#[test]
fn casm_add_pythonic_hints() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            sierra = false
            casm = true
            casm-add-pythonic-hints = true
        "#})
        .dep_starknet()
        .lib_cairo(BALANCE_CONTRACT.as_str())
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        [..]  Finished release target(s) in [..]
        "#});

    assert_is_json::<CasmContractClass>(&t.child("target/dev/hello_Balance.casm.json"));
}

#[test]
fn compile_starknet_contract_only_with_cfg() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [lib]

            [[target.starknet-contract]]
        "#})
        .dep_starknet()
        .lib_cairo(formatdoc! {r#"
            #[cfg(target: 'starknet-contract')]
            {BALANCE_CONTRACT_INTERFACE}

            #[cfg(target: 'starknet-contract')]
            {BALANCE_CONTRACT_IMPLEMENTATION}
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    assert_eq!(
        t.child("target/dev").files(),
        vec![
            "hello.sierra",
            "hello.starknet_artifacts.json",
            "hello_Balance.sierra.json"
        ]
    );

    t.child("target/dev/hello.sierra")
        .assert(predicates::str::contains("hello::Balance::balance::read").not());

    assert_is_json::<ContractClass>(&t.child("target/dev/hello_Balance.sierra.json"));
}

#[test]
fn compile_starknet_contract_without_starknet_dep() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra("[[target.starknet-contract]]")
        .lib_cairo(BALANCE_CONTRACT.as_str())
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        warn: package `hello` declares `starknet-contract` target, but does not depend on `starknet` package
        note: this may cause contract compilation to fail with cryptic errors
        help: add dependency on `starknet` to package manifest
         --> Scarb.toml
            [dependencies]
            starknet = ">=[..]"

        error: Type not found.
         --> lib.cairo:19:30
            fn constructor(ref self: ContractState, value_: u128) {
                                     ^***********^

        error: Method `write` not found on type "<missing>". Did you import the correct trait and impl?
         --> lib.cairo:20:20
                self.value.write(value_);
                           ^***^

        error: Type not found.
         --> lib.cairo:24:37
            impl Balance of super::IBalance<ContractState> {
                                            ^***********^

        error: Type not found.
         --> lib.cairo:25:23
                fn get(self: @ContractState) -> u128 {
                              ^***********^

        error: Method `read` not found on type "<missing>". Did you import the correct trait and impl?
         --> lib.cairo:26:24
                    self.value.read()
                               ^**^

        error: Type not found.
         --> lib.cairo:28:31
                fn increase(ref self: ContractState, a: u128)  {
                                      ^***********^

        error: Method `write` not found on type "<missing>". Did you import the correct trait and impl?
         --> lib.cairo:29:24
                    self.value.write( self.value.read() + a );
                               ^***^


        error: could not compile `hello` due to previous error
        "#});
}

fn compile_dep_test_case(hello: &ChildPath, world: &ChildPath, target_extra: &str) {
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [lib]
            [[target.starknet-contract]]
        "#})
        .dep_starknet()
        .lib_cairo(format!("{}\n{}", BALANCE_CONTRACT.as_str(), HELLO_CONTRACT))
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
fn do_not_compile_dep_contracts() {
    let t = assert_fs::TempDir::new().unwrap();
    let hello = t.child("hello");
    let world = t.child("world");
    compile_dep_test_case(&hello, &world, "");

    assert_eq!(
        world
            .child("target/dev")
            .files()
            .iter()
            .sorted()
            .collect::<Vec<&String>>(),
        vec![
            "world.starknet_artifacts.json",
            "world_FortyTwo.sierra.json",
            "world_HelloContract.sierra.json",
        ]
    );
    assert_is_json::<ContractClass>(&world.child("target/dev/world_FortyTwo.sierra.json"));
    assert_is_json::<ContractClass>(&world.child("target/dev/world_HelloContract.sierra.json"));
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
    assert_is_json::<ContractClass>(&world.child("target/dev/world_Balance.sierra.json"));
    assert_is_json::<ContractClass>(&world.child("target/dev/world_FortyTwo.sierra.json"));
    assert_is_json::<ContractClass>(&world.child("target/dev/world_HelloContract.sierra.json"));
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
    assert_is_json::<ContractClass>(&world.child("target/dev/world_Balance.sierra.json"));
    assert_is_json::<ContractClass>(
        &world.child("target/dev/world_hello_HelloContract.sierra.json"),
    );
    assert_is_json::<ContractClass>(&world.child("target/dev/world_FortyTwo.sierra.json"));
    assert_is_json::<ContractClass>(
        &world.child("target/dev/world_hello_HelloContract.sierra.json"),
    );

    // Check starknet artifacts content
    let starknet_artifacts = &world.child("target/dev/world.starknet_artifacts.json");
    assert_is_json::<serde_json::Value>(starknet_artifacts);
    let content = fs::read_to_string(starknet_artifacts).unwrap();
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
            format!("{}\n{}", BALANCE_CONTRACT.as_str(), HELLO_CONTRACT),
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
