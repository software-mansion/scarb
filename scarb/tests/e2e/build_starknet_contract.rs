use std::fs::File;
use std::io::BufReader;

use assert_fs::fixture::ChildPath;
use assert_fs::prelude::*;
use cairo_lang_starknet::casm_contract_class::CasmContractClass;
use cairo_lang_starknet::contract_class::ContractClass;
use indoc::{formatdoc, indoc};
use predicates::prelude::*;
use serde::de::DeserializeOwned;

use crate::support::command::Scarb;
use crate::support::fsx::ChildPathEx;
use crate::support::project_builder::ProjectBuilder;

const BALANCE_CONTRACT: &str = indoc! {r#"
    #[contract]
    mod Balance {
        struct Storage {
            balance: felt252,
        }

        // Increases the balance by the given amount.
        #[external]
        fn increase_balance(amount: felt252) {
            balance::write(balance::read() + amount);
        }

        // Returns the current balance.
        #[view]
        fn get_balance() -> felt252 {
            balance::read()
        }
    }
"#};

const FORTY_TWO_CONTRACT: &str = indoc! {r#"
    #[contract]
    mod FortyTwo {
        #[external]
        fn answer() -> felt252 { 42 }
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
        .lib_cairo(BALANCE_CONTRACT)
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
        .lib_cairo(BALANCE_CONTRACT)
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
        .src("src/balance.cairo", BALANCE_CONTRACT)
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
            "a_Balance.sierra.json",
            "a_FortyTwo.sierra.json",
            "b_Balance.sierra.json",
            "b_FortyTwo.sierra.json",
            "hello.casm",
            "hello.sierra",
            "hello.starknet_artifacts.json"
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

    assert_is_json::<serde_json::Value>(&t.child("target/dev/hello.starknet_artifacts.json"));
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
        .lib_cairo(BALANCE_CONTRACT)
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
            {BALANCE_CONTRACT}
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
        .lib_cairo(BALANCE_CONTRACT)
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

        error: Identifier not found.
         --> lib.cairo:10:9
                balance::write(balance::read() + amount);
                ^*****^

        error: Identifier not found.
         --> lib.cairo:16:9
                balance::read()
                ^*****^


        error: could not compile `hello` due to previous error
        "#});
}
