use assert_fs::fixture::ChildPath;
use assert_fs::prelude::*;
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use cairo_lang_starknet_classes::contract_class::ContractClass;
use indoc::{formatdoc, indoc};
use itertools::Itertools;
use predicates::prelude::*;

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
            [..]  Finished `dev` profile target(s) in [..]
        "#});
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
        [..]  Finished `dev` profile target(s) in [..]
        "#});

    assert_eq!(
        t.child("target/dev").files(),
        vec![
            "hello.starknet_artifacts.json",
            "hello_Balance.contract_class.json"
        ]
    );

    t.child("target/dev/hello_Balance.contract_class.json")
        .assert_is_json::<ContractClass>();
    t.child("target/dev/hello.starknet_artifacts.json")
        .assert(predicates::str::contains(
            r#""module_path":"hello::Balance""#,
        ));
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
        [..]  Finished `dev` profile target(s) in [..]
        "#});

    assert_eq!(
        t.child("target/dev").files(),
        vec![
            "hello.starknet_artifacts.json",
            "hello_Balance.compiled_contract_class.json"
        ]
    );

    t.child("target/dev/hello_Balance.compiled_contract_class.json")
        .assert_is_json::<CasmContractClass>();
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
            sierra-text = true

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
        [..]  Finished `dev` profile target(s) in [..]
        "#});

    assert_eq!(
        t.child("target/dev").files(),
        vec![
            "a.starknet_artifacts.json",
            "a_Balance.contract_class.json",
            "a_FortyTwo.contract_class.json",
            "b.starknet_artifacts.json",
            "b_Balance.contract_class.json",
            "b_FortyTwo.contract_class.json",
            "hello.casm",
            "hello.sierra",
            "hello.sierra.json",
        ]
    );

    for json in [
        "a_Balance.contract_class.json",
        "a_FortyTwo.contract_class.json",
        "b_Balance.contract_class.json",
        "b_FortyTwo.contract_class.json",
    ] {
        t.child("target/dev")
            .child(json)
            .assert_is_json::<ContractClass>();
    }

    t.child("target/dev/a.starknet_artifacts.json")
        .assert_is_json::<serde_json::Value>();
    t.child("target/dev/b.starknet_artifacts.json")
        .assert_is_json::<serde_json::Value>();
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
        [..]  Finished `dev` profile target(s) in [..]
        "#});

    assert_eq!(
        t.child("target/dev").files(),
        vec![
            "hello.starknet_artifacts.json",
            "hello_hello_forty_two_FortyTwo.contract_class.json",
            "hello_hello_world_FortyTwo.contract_class.json",
        ]
    );

    t.child("target/dev/hello.starknet_artifacts.json")
        .assert_is_json::<serde_json::Value>();
    t.child("target/dev/hello_hello_forty_two_FortyTwo.contract_class.json")
        .assert_is_json::<serde_json::Value>();
    t.child("target/dev/hello_hello_world_FortyTwo.contract_class.json")
        .assert_is_json::<serde_json::Value>();
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
        [..]  Finished `dev` profile target(s) in [..]
        "#});

    t.child("target/dev/hello_Balance.compiled_contract_class.json")
        .assert_is_json::<CasmContractClass>();
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
        .lib_cairo(indoc! {r#"
            #[cfg(target: 'starknet-contract')]
            #[starknet::interface]
            trait IBalance<T> {
                // Returns the current balance.
                fn get(self: @T) -> u128;
                // Increases the balance by the given amount.
                fn increase(ref self: T, a: u128);
            }

            #[cfg(target: 'starknet-contract')]
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

                #[abi(embed_v0)]
                impl Balance of super::IBalance<ContractState> {
                    fn get(self: @ContractState) -> u128 {
                        self.value.read()
                    }
                    fn increase(ref self: ContractState, a: u128)  {
                        self.value.write( self.value.read() + a );
                    }
                }
            }
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
            "hello.sierra.json",
            "hello.starknet_artifacts.json",
            "hello_Balance.contract_class.json"
        ]
    );

    t.child("target/dev/hello.sierra.json")
        .assert(predicates::str::contains("hello::Balance::balance::read").not());

    t.child("target/dev/hello_Balance.contract_class.json")
        .assert_is_json::<ContractClass>();
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
        .arg("check")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Checking hello v0.1.0 ([..])
        warn: package `hello` declares `starknet-contract` target, but does not depend on `starknet` package
        note: this may cause contract compilation to fail with cryptic errors
        help: add dependency on `starknet` to package manifest
         --> Scarb.toml
            [dependencies]
            starknet = ">=[..]"

        error: Plugin diagnostic: Unsupported attribute.
         --> [..]src/lib.cairo:9:1
        #[starknet::contract]
        ^*******************^

        error: Plugin diagnostic: Unsupported attribute.
         --> [..]src/lib.cairo:13:5
            #[storage]
            ^********^

        error: Plugin diagnostic: Unsupported attribute.
         --> [..]src/lib.cairo:18:5
            #[constructor]
            ^************^

        error: Plugin diagnostic: Unsupported attribute.
         --> [..]src/lib.cairo:23:5
            #[abi(embed_v0)]
            ^**************^

        error: Type not found.
         --> [..]src/lib.cairo:19:30
            fn constructor(ref self: ContractState, value_: u128) {
                                     ^***********^

        error: Ambiguous method call. More than one applicable trait function with a suitable self type was found: core::starknet::storage::map::StorageMapWriteAccess::write and core::starknet::storage::StoragePointerWriteAccess::write. Consider adding type annotations or explicitly refer to the impl function.
         --> [..]src/lib.cairo:20:20
                self.value.write(value_);
                           ^***^

        error: Type not found.
         --> [..]src/lib.cairo:24:37
            impl Balance of super::IBalance<ContractState> {
                                            ^***********^

        error: Type not found.
         --> [..]src/lib.cairo:25:23
                fn get(self: @ContractState) -> u128 {
                              ^***********^

        error: Ambiguous method call. More than one applicable trait function with a suitable self type was found: core::starknet::storage::map::StorageMapReadAccess::read and core::starknet::storage::StoragePointerReadAccess::read. Consider adding type annotations or explicitly refer to the impl function.
         --> [..]src/lib.cairo:26:24
                    self.value.read()
                               ^**^

        error: Type not found.
         --> [..]src/lib.cairo:28:31
                fn increase(ref self: ContractState, a: u128)  {
                                      ^***********^

        error: Ambiguous method call. More than one applicable trait function with a suitable self type was found: core::starknet::storage::map::StorageMapWriteAccess::write and core::starknet::storage::StoragePointerWriteAccess::write. Consider adding type annotations or explicitly refer to the impl function.
         --> [..]src/lib.cairo:29:24
                    self.value.write( self.value.read() + a );
                               ^***^

        error: could not check `hello` due to previous error
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
            "world_FortyTwo.contract_class.json",
            "world_HelloContract.contract_class.json",
        ]
    );
    world
        .child("target/dev/world_FortyTwo.contract_class.json")
        .assert_is_json::<ContractClass>();
    world
        .child("target/dev/world_HelloContract.contract_class.json")
        .assert_is_json::<ContractClass>();
}
