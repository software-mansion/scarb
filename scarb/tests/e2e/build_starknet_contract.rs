use std::fs;

use assert_fs::fixture::ChildPath;
use assert_fs::prelude::*;
use cairo_lang_starknet::contract_class::ContractClass;
use indoc::indoc;

use crate::support::command::Scarb;
use crate::support::fsx::ChildPathEx;
use crate::support::project_builder::ProjectBuilder;

#[test]
fn compile_starknet_contract() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra("[[target.starknet-contract]]")
        .lib_cairo(indoc! {r#"
            #[contract]
            mod HelloStarknet {
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
        "#})
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
        t.child("target/release").files(),
        vec!["hello_HelloStarknet.json"]
    );

    assert_is_contract_class(&t.child("target/release/hello_HelloStarknet.json"));
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
        .lib_cairo(indoc! {r#"
            mod hello;
            mod foo;
        "#})
        .src(
            "src/hello.cairo",
            indoc! {r#"
                #[contract]
                mod Hello {
                    #[external]
                    fn hello() {}
                }
            "#},
        )
        .src(
            "src/foo.cairo",
            indoc! {r#"
                #[contract]
                mod Foo {
                    #[external]
                    fn foo() -> felt252 { 42 }
                }
            "#},
        )
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
        t.child("target/release").files(),
        vec![
            "a_Foo.json",
            "a_Hello.json",
            "b_Foo.json",
            "b_Hello.json",
            "hello.casm",
            "hello.sierra",
        ]
    );

    for json in ["a_Foo.json", "a_Hello.json", "b_Foo.json", "b_Hello.json"] {
        assert_is_contract_class(&t.child("target/release").child(json));
    }
}

fn assert_is_contract_class(child: &ChildPath) {
    let contract_json = fs::read_to_string(child.path()).unwrap();
    serde_json::from_str::<ContractClass>(&contract_json).unwrap();
}
