use assert_fs::TempDir;
use assert_fs::prelude::*;
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use cairo_lang_starknet_classes::contract_class::ContractClass;
use cairo_lang_utils::bigint::BigUintAsHex;
use indoc::indoc;

use scarb_test_support::command::Scarb;
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn compile_static_forwarding_contract() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            forwarding = true
        "#})
        .dep_starknet()
        .lib_cairo(indoc! {r#"
            #[starknet::interface]
            pub trait ICounterContract<TContractState> {
                fn increase_counter(ref self: TContractState, amount: u128);
                fn get_counter(self: @TContractState) -> u128;
            }

            #[starknet::contract]
            pub mod counter_contract {
                use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};

                #[storage]
                struct Storage {
                    counter: u128,
                }

                #[abi(embed_v0)]
                impl ICounterImpl of super::ICounterContract<ContractState> {
                    fn increase_counter(ref self: ContractState, amount: u128) {
                        self.counter.write(self.counter.read() + amount);
                    }

                    fn get_counter(self: @ContractState) -> u128 {
                        self.counter.read()
                    }
                }
            }

            #[starknet::contract]
            #[feature("forward-impl")]
            pub mod static_proxy {
                #[storage]
                struct Storage {}

                impl CounterClassHash =
                    super::counter_contract__class_hash__::ForwardingClassHashImpl<ContractState>;

                #[abi(embed_v0)]
                impl ForwardedImpl = super::ICounterContractForwardImpl<ContractState>;
            }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        [..]  Finished `dev` profile target(s) in [..]
        "#});

    assert_eq!(
        t.child("target/dev").files(),
        vec![
            ".fingerprint",
            "hello.starknet_artifacts.json",
            "hello_counter_contract.contract_class.json",
            "hello_static_proxy.contract_class.json",
            "incremental",
        ]
    );

    t.child("target/dev/hello_counter_contract.contract_class.json")
        .assert_is_json::<ContractClass>();
    let proxy = t
        .child("target/dev/hello_static_proxy.contract_class.json")
        .assert_is_json::<ContractClass>();
    assert_eq!(proxy.entry_points_by_type.external.len(), 2);
}

// The embedded constant must be the target's *real* class hash, not just "some number that
// happens to change" - computed the same way a real `declare` transaction would.
#[test]
fn forwarding_embeds_the_real_target_class_hash() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            forwarding = true
        "#})
        .dep_starknet()
        .lib_cairo(indoc! {r#"
            #[starknet::interface]
            pub trait ICounterContract<TContractState> {
                fn increase_counter(ref self: TContractState, amount: u128);
                fn get_counter(self: @TContractState) -> u128;
            }

            #[starknet::contract]
            pub mod counter_contract {
                use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};
                #[storage]
                struct Storage { counter: u128 }
                #[abi(embed_v0)]
                impl ICounterImpl of super::ICounterContract<ContractState> {
                    fn increase_counter(ref self: ContractState, amount: u128) {
                        self.counter.write(self.counter.read() + amount);
                    }
                    fn get_counter(self: @ContractState) -> u128 { self.counter.read() }
                }
            }

            #[starknet::contract]
            #[feature("forward-impl")]
            pub mod static_proxy {
                #[storage] struct Storage {}
                impl CounterClassHash =
                    super::counter_contract__class_hash__::ForwardingClassHashImpl<ContractState>;
                #[abi(embed_v0)]
                impl ForwardedImpl = super::ICounterContractForwardImpl<ContractState>;
            }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    let counter = t
        .child("target/dev/hello_counter_contract.contract_class.json")
        .assert_is_json::<ContractClass>();
    let proxy = t
        .child("target/dev/hello_static_proxy.contract_class.json")
        .assert_is_json::<ContractClass>();

    let real_hash = BigUintAsHex::from(class_hash(&counter).to_biguint());
    assert!(
        proxy.sierra_program.contains(&real_hash),
        "proxy's compiled program should contain counter_contract's real class hash"
    );
}

#[test]
fn compile_multi_forwarding_contract_with_forwarding_dependency() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            forwarding = true
        "#})
        .dep_starknet()
        .lib_cairo(indoc! {r#"
            #[starknet::interface]
            pub trait ICounter<TContractState> {
                fn increase_counter(ref self: TContractState, amount: u128);
                fn get_counter(self: @TContractState) -> u128;
            }

            #[starknet::interface]
            pub trait IFlag<TContractState> {
                fn set_flag(ref self: TContractState, value: felt252);
                fn get_flag(self: @TContractState) -> felt252;
            }

            #[starknet::interface]
            pub trait IStats<TContractState> {
                fn get_stats(self: @TContractState) -> felt252;
            }

            #[starknet::contract]
            pub mod counter_contract {
                use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};

                #[storage]
                struct Storage {
                    counter: u128,
                }

                #[abi(embed_v0)]
                impl CounterImpl of super::ICounter<ContractState> {
                    fn increase_counter(ref self: ContractState, amount: u128) {
                        self.counter.write(self.counter.read() + amount);
                    }

                    fn get_counter(self: @ContractState) -> u128 {
                        self.counter.read()
                    }
                }
            }

            #[starknet::contract]
            pub mod flag_contract {
                use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};

                #[storage]
                struct Storage {
                    flag: felt252,
                }

                #[abi(embed_v0)]
                impl FlagImpl of super::IFlag<ContractState> {
                    fn set_flag(ref self: ContractState, value: felt252) {
                        self.flag.write(value);
                    }

                    fn get_flag(self: @ContractState) -> felt252 {
                        self.flag.read()
                    }
                }
            }

            #[starknet::contract]
            #[feature("forward-impl")]
            pub mod stats_proxy {
                #[storage]
                struct Storage {}

                impl CounterClassHash =
                    super::counter_contract__class_hash__::ForwardingClassHashImpl<ContractState>;

                #[abi(embed_v0)]
                impl ForwardedCounterImpl = super::ICounterForwardImpl<ContractState>;

                #[abi(embed_v0)]
                impl StatsImpl of super::IStats<ContractState> {
                    fn get_stats(self: @ContractState) -> felt252 {
                        7
                    }
                }
            }

            #[starknet::contract]
            #[feature("forward-impl")]
            pub mod multi_proxy {
                #[storage]
                struct Storage {}

                impl StatsClassHash =
                    super::stats_proxy__class_hash__::ForwardingClassHashImpl<ContractState>;
                impl FlagClassHash =
                    super::flag_contract__class_hash__::ForwardingClassHashImpl<ContractState>;

                #[abi(embed_v0)]
                impl ForwardedStatsImpl =
                    super::IStatsForwardImpl<ContractState, StatsClassHash>;

                #[abi(embed_v0)]
                impl ForwardedFlagImpl =
                    super::IFlagForwardImpl<ContractState, FlagClassHash>;
            }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        [..]  Finished `dev` profile target(s) in [..]
        "#});

    assert_eq!(
        t.child("target/dev").files(),
        vec![
            ".fingerprint",
            "hello.starknet_artifacts.json",
            "hello_counter_contract.contract_class.json",
            "hello_flag_contract.contract_class.json",
            "hello_multi_proxy.contract_class.json",
            "hello_stats_proxy.contract_class.json",
            "incremental",
        ]
    );

    let stats_proxy = t
        .child("target/dev/hello_stats_proxy.contract_class.json")
        .assert_is_json::<ContractClass>();
    assert_eq!(stats_proxy.entry_points_by_type.external.len(), 3);

    let multi_proxy = t
        .child("target/dev/hello_multi_proxy.contract_class.json")
        .assert_is_json::<ContractClass>();
    assert_eq!(multi_proxy.entry_points_by_type.external.len(), 3);
}

// Regression: `lib` targets used to panic on any contract, even without forwarding.

#[test]
fn compile_multiple_contracts_without_forwarding() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
        "#})
        .dep_starknet()
        .lib_cairo(indoc! {r#"
            #[starknet::contract]
            pub mod first_contract {
                #[storage]
                struct Storage {}
            }

            #[starknet::contract]
            pub mod second_contract {
                #[storage]
                struct Storage {}
            }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        [..]  Finished `dev` profile target(s) in [..]
        "#});

    t.child("target/dev/hello_first_contract.contract_class.json")
        .assert_is_json::<ContractClass>();
    t.child("target/dev/hello_second_contract.contract_class.json")
        .assert_is_json::<ContractClass>();
}

#[test]
fn compile_test_target_with_contract_without_forwarding() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .dep_starknet()
        .dep_cairo_test()
        .lib_cairo(indoc! {r#"
            #[starknet::contract]
            pub mod plain_contract {
                #[storage]
                struct Storage {}
            }

            #[cfg(test)]
            mod tests {
                #[test]
                fn it_works() {
                    assert!(true);
                }
            }
        "#})
        .build(&t);

    // `--test` compiles without running, so this doesn't need a real cairo-test binary.
    Scarb::quick_command()
        .arg("build")
        .arg("--test")
        .current_dir(&t)
        .assert()
        .success();

    let content = t
        .child("target/dev/hello_unittest.test.json")
        .read_to_string();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    let tests = json.get("named_tests").unwrap().as_array().unwrap();
    assert_eq!(tests.len(), 1);
}

#[test]
fn compile_lib_target_with_contract_without_forwarding() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.lib]]
        "#})
        .dep_starknet()
        .lib_cairo(indoc! {r#"
            #[starknet::contract]
            pub mod plain_contract {
                #[storage]
                struct Storage {}
            }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        [..]  Finished `dev` profile target(s) in [..]
        "#});
}

// Regression: forwarding without `forwarding = true` used to silently embed `0` instead of erroring.

const FORWARDING_CONTRACTS: &str = indoc! {r#"
    #[starknet::interface]
    pub trait ICounterContract<TContractState> {
        fn increase_counter(ref self: TContractState, amount: u128);
        fn get_counter(self: @TContractState) -> u128;
    }

    #[starknet::contract]
    pub mod counter_contract {
        use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};
        #[storage]
        struct Storage { counter: u128 }
        #[abi(embed_v0)]
        impl ICounterImpl of super::ICounterContract<ContractState> {
            fn increase_counter(ref self: ContractState, amount: u128) {
                self.counter.write(self.counter.read() + amount);
            }
            fn get_counter(self: @ContractState) -> u128 { self.counter.read() }
        }
    }

    #[starknet::contract]
    #[feature("forward-impl")]
    pub mod static_proxy {
        #[storage]
        struct Storage {}
        impl CounterClassHash =
            super::counter_contract__class_hash__::ForwardingClassHashImpl<ContractState>;
        #[abi(embed_v0)]
        impl ForwardedImpl = super::ICounterContractForwardImpl<ContractState>;
    }
"#};

#[test]
fn starknet_contract_without_forwarding_flag_errors_instead_of_silently_wrong() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
        "#})
        .dep_starknet()
        .lib_cairo(FORWARDING_CONTRACTS)
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        error: contract(s) use static forwarding but it did not run for this build: hello::counter_contract. Set `forwarding = true` on this target.
        error: could not compile `hello` due to 1 previous error
        "#});
}

#[test]
fn test_target_cannot_use_forwarding() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .dep_starknet()
        .dep_cairo_test()
        .lib_cairo(FORWARDING_CONTRACTS)
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .arg("--test")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
        [..]
        error: contract(s) use static forwarding but it did not run for this build: hello::counter_contract. Static forwarding is not supported in test targets.
        [..]
        "#});
}

#[test]
fn executable_target_cannot_use_forwarding() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .dep_starknet()
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [[target.executable]]

            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(format!(
            "{FORWARDING_CONTRACTS}\n{}",
            indoc! {r#"
            #[executable]
            fn main() -> starknet::ClassHash {
                counter_contract__class_hash__::class_hash()
            }
        "#}
        ))
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
        [..]
        error: contract(s) use static forwarding but it did not run for this build: hello::counter_contract. Static forwarding is not supported in executable targets.
        [..]
        "#});
}

// Regression: two starknet-contract targets in one package used to abort the whole process.
#[test]
fn two_starknet_contract_targets_in_one_package_do_not_abort() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            name = "a"

            [[target.starknet-contract]]
            name = "b"
        "#})
        .dep_starknet()
        .lib_cairo(indoc! {r#"
            #[starknet::contract]
            pub mod plain_contract {
                #[storage]
                struct Storage {}
            }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();
}

// Same regression, but with both targets actually using forwarding.
#[test]
fn two_forwarding_enabled_targets_with_real_usage_do_not_abort() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            name = "a"
            forwarding = true

            [[target.starknet-contract]]
            name = "b"
            forwarding = true
        "#})
        .dep_starknet()
        .lib_cairo(FORWARDING_CONTRACTS)
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();
}

// The three central forwarding error paths: cycle, self-reference, unknown target.

#[test]
fn forwarding_cycle_errors() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            forwarding = true
        "#})
        .dep_starknet()
        .lib_cairo(indoc! {r#"
            #[starknet::interface]
            pub trait IA<T> { fn a(self: @T) -> felt252; }
            #[starknet::interface]
            pub trait IB<T> { fn b(self: @T) -> felt252; }

            #[starknet::contract]
            #[feature("forward-impl")]
            pub mod contract_a {
                #[storage] struct Storage {}
                impl BClassHash = super::contract_b__class_hash__::ForwardingClassHashImpl<ContractState>;
                #[abi(embed_v0)]
                impl ForwardedA = super::IBForwardImpl<ContractState, BClassHash>;
            }

            #[starknet::contract]
            #[feature("forward-impl")]
            pub mod contract_b {
                #[storage] struct Storage {}
                impl AClassHash = super::contract_a__class_hash__::ForwardingClassHashImpl<ContractState>;
                #[abi(embed_v0)]
                impl ForwardedB = super::IAForwardImpl<ContractState, AClassHash>;
            }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
        [..]
        error: forwarding cycle detected among contracts: hello::contract_a, hello::contract_b
        [..]
        "#});
}

#[test]
fn forwarding_self_reference_errors() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            forwarding = true
        "#})
        .dep_starknet()
        .lib_cairo(indoc! {r#"
            #[starknet::interface]
            pub trait ISelf<T> { fn f(self: @T) -> felt252; }

            #[starknet::contract]
            #[feature("forward-impl")]
            pub mod self_contract {
                #[storage] struct Storage {}
                impl SelfClassHash = super::self_contract__class_hash__::ForwardingClassHashImpl<ContractState>;
                #[abi(embed_v0)]
                impl Forwarded = super::ISelfForwardImpl<ContractState, SelfClassHash>;
            }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
        [..]
        error: contract `hello::self_contract` cannot statically forward to its own class hash[..]
        [..]
        "#});
}

// A typo'd target path is caught by Cairo's own name resolution, not Scarb's.
#[test]
fn forwarding_target_typo_errors_via_cairo() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            forwarding = true
        "#})
        .dep_starknet()
        .lib_cairo(indoc! {r#"
            #[starknet::interface]
            pub trait ICounter<T> {
                fn increase_counter(ref self: T, amount: u128);
                fn get_counter(self: @T) -> u128;
            }

            #[starknet::contract]
            #[feature("forward-impl")]
            pub mod orphan_proxy {
                #[storage] struct Storage {}
                impl CounterClassHash =
                    super::nonexistent_contract__class_hash__::ForwardingClassHashImpl<ContractState>;
                #[abi(embed_v0)]
                impl Forwarded = super::ICounterForwardImpl<ContractState, CounterClassHash>;
            }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
        [..]
        error[E0006]: Identifier not found.
        [..]
                super::nonexistent_contract__class_hash__::ForwardingClassHashImpl<ContractState>;
                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

        error: could not compile `hello` due to 1 previous error
        "#});
}

// Cross-package forwarding via `build-external-contracts`.
#[test]
fn forwarding_across_packages_succeeds() {
    let t = TempDir::new().unwrap();
    let hello = t.child("hello");
    let world = t.child("world");

    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [lib]
            [[target.starknet-contract]]
        "#})
        .dep_starknet()
        .lib_cairo(indoc! {r#"
            #[starknet::interface]
            pub trait ICounter<T> {
                fn increase_counter(ref self: T, amount: u128);
                fn get_counter(self: @T) -> u128;
            }

            #[starknet::contract]
            pub mod counter_contract {
                use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};
                #[storage]
                struct Storage { counter: u128 }
                #[abi(embed_v0)]
                impl CounterImpl of super::ICounter<ContractState> {
                    fn increase_counter(ref self: ContractState, amount: u128) {
                        self.counter.write(self.counter.read() + amount);
                    }
                    fn get_counter(self: @ContractState) -> u128 { self.counter.read() }
                }
            }
        "#})
        .build(&hello);

    ProjectBuilder::start()
        .name("world")
        .edition("2023_01")
        .version("0.1.0")
        .dep("hello", &hello)
        .dep_starknet()
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            forwarding = true
            build-external-contracts = ["hello::counter_contract"]
        "#})
        .lib_cairo(indoc! {r#"
            #[starknet::contract]
            #[feature("forward-impl")]
            pub mod proxy_contract {
                #[storage]
                struct Storage {}
                impl CounterClassHash =
                    hello::counter_contract__class_hash__::ForwardingClassHashImpl<ContractState>;
                #[abi(embed_v0)]
                impl Forwarded = hello::ICounterForwardImpl<ContractState>;
            }
        "#})
        .build(&world);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&world)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..] Compiling world v0.1.0 ([..])
        [..]  Finished `dev` profile target(s) in [..]
        "#});

    let proxy = world
        .child("target/dev/world_proxy_contract.contract_class.json")
        .assert_is_json::<ContractClass>();
    assert_eq!(proxy.entry_points_by_type.external.len(), 2);
}

// Same setup, but the target is missing from `build-external-contracts` - Scarb's own error.
#[test]
fn forwarding_target_not_in_build_external_contracts_errors() {
    let t = TempDir::new().unwrap();
    let hello = t.child("hello");
    let world = t.child("world");

    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [lib]
            [[target.starknet-contract]]
        "#})
        .dep_starknet()
        .lib_cairo(indoc! {r#"
            #[starknet::interface]
            pub trait ICounter<T> {
                fn increase_counter(ref self: T, amount: u128);
                fn get_counter(self: @T) -> u128;
            }

            #[starknet::contract]
            pub mod counter_contract {
                use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};
                #[storage]
                struct Storage { counter: u128 }
                #[abi(embed_v0)]
                impl CounterImpl of super::ICounter<ContractState> {
                    fn increase_counter(ref self: ContractState, amount: u128) {
                        self.counter.write(self.counter.read() + amount);
                    }
                    fn get_counter(self: @ContractState) -> u128 { self.counter.read() }
                }
            }
        "#})
        .build(&hello);

    ProjectBuilder::start()
        .name("world")
        .edition("2023_01")
        .version("0.1.0")
        .dep("hello", &hello)
        .dep_starknet()
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            forwarding = true
        "#})
        .lib_cairo(indoc! {r#"
            #[starknet::contract]
            #[feature("forward-impl")]
            pub mod proxy_contract {
                #[storage]
                struct Storage {}
                impl CounterClassHash =
                    hello::counter_contract__class_hash__::ForwardingClassHashImpl<ContractState>;
                #[abi(embed_v0)]
                impl Forwarded = hello::ICounterForwardImpl<ContractState>;
            }
        "#})
        .build(&world);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&world)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
        [..] Compiling world v0.1.0 ([..])
        error: contract `world::proxy_contract` forwards to contract(s) that are not included in this build: hello::counter_contract. Add them with `build-external-contracts` or fix the forwarding target.
        error: could not compile `world` due to 1 previous error
        "#});
}

// Two facades forwarding to the same target must embed the same hash.
#[test]
fn forwarding_diamond_dependency() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            forwarding = true
        "#})
        .dep_starknet()
        .lib_cairo(indoc! {r#"
            #[starknet::interface]
            pub trait ICounter<T> {
                fn increase_counter(ref self: T, amount: u128);
                fn get_counter(self: @T) -> u128;
            }

            #[starknet::contract]
            pub mod counter_contract {
                use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};
                #[storage]
                struct Storage { counter: u128 }
                #[abi(embed_v0)]
                impl CounterImpl of super::ICounter<ContractState> {
                    fn increase_counter(ref self: ContractState, amount: u128) {
                        self.counter.write(self.counter.read() + amount);
                    }
                    fn get_counter(self: @ContractState) -> u128 { self.counter.read() }
                }
            }

            #[starknet::contract]
            #[feature("forward-impl")]
            pub mod proxy_a {
                #[storage] struct Storage {}
                impl CounterClassHash =
                    super::counter_contract__class_hash__::ForwardingClassHashImpl<ContractState>;
                #[abi(embed_v0)]
                impl Forwarded = super::ICounterForwardImpl<ContractState>;
            }

            #[starknet::contract]
            #[feature("forward-impl")]
            pub mod proxy_b {
                #[storage] struct Storage {}
                impl CounterClassHash =
                    super::counter_contract__class_hash__::ForwardingClassHashImpl<ContractState>;
                #[abi(embed_v0)]
                impl Forwarded = super::ICounterForwardImpl<ContractState>;
            }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    let proxy_a = t
        .child("target/dev/hello_proxy_a.contract_class.json")
        .assert_is_json::<ContractClass>();
    let proxy_b = t
        .child("target/dev/hello_proxy_b.contract_class.json")
        .assert_is_json::<ContractClass>();
    assert_eq!(proxy_a.entry_points_by_type.external.len(), 2);
    assert_eq!(proxy_b.entry_points_by_type.external.len(), 2);
    // Compare compiled programs, not selectors (selectors don't depend on the embedded hash).
    assert_eq!(proxy_a.sierra_program, proxy_b.sierra_program);
}

#[test]
fn forwarding_three_way_cycle_errors() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            forwarding = true
        "#})
        .dep_starknet()
        .lib_cairo(indoc! {r#"
            #[starknet::interface]
            pub trait IA<T> { fn a(self: @T) -> felt252; }
            #[starknet::interface]
            pub trait IB<T> { fn b(self: @T) -> felt252; }
            #[starknet::interface]
            pub trait IC<T> { fn c(self: @T) -> felt252; }

            #[starknet::contract]
            #[feature("forward-impl")]
            pub mod contract_a {
                #[storage] struct Storage {}
                impl BClassHash = super::contract_b__class_hash__::ForwardingClassHashImpl<ContractState>;
                #[abi(embed_v0)]
                impl Forwarded = super::IBForwardImpl<ContractState, BClassHash>;
            }

            #[starknet::contract]
            #[feature("forward-impl")]
            pub mod contract_b {
                #[storage] struct Storage {}
                impl CClassHash = super::contract_c__class_hash__::ForwardingClassHashImpl<ContractState>;
                #[abi(embed_v0)]
                impl Forwarded = super::ICForwardImpl<ContractState, CClassHash>;
            }

            #[starknet::contract]
            #[feature("forward-impl")]
            pub mod contract_c {
                #[storage] struct Storage {}
                impl AClassHash = super::contract_a__class_hash__::ForwardingClassHashImpl<ContractState>;
                #[abi(embed_v0)]
                impl Forwarded = super::IAForwardImpl<ContractState, AClassHash>;
            }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        error: forwarding cycle detected among contracts: hello::contract_a, hello::contract_b, hello::contract_c
        error: could not compile `hello` due to 1 previous error
        "#});
}

// Changing the target's source must change the facade's embedded hash on rebuild.
#[test]
fn forwarding_survives_incremental_rebuild() {
    let t = assert_fs::TempDir::new().unwrap();
    let manifest = indoc! {r#"
        [[target.starknet-contract]]
        forwarding = true
    "#};
    let lib_cairo = |amount_expr: &str| {
        format!(
            r#"
            #[starknet::interface]
            pub trait ICounter<T> {{
                fn increase_counter(ref self: T, amount: u128);
                fn get_counter(self: @T) -> u128;
            }}

            #[starknet::contract]
            pub mod counter_contract {{
                use starknet::storage::{{StoragePointerReadAccess, StoragePointerWriteAccess}};
                #[storage]
                struct Storage {{ counter: u128 }}
                #[abi(embed_v0)]
                impl CounterImpl of super::ICounter<ContractState> {{
                    fn increase_counter(ref self: ContractState, amount: u128) {{
                        self.counter.write({amount_expr});
                    }}
                    fn get_counter(self: @ContractState) -> u128 {{ self.counter.read() }}
                }}
            }}

            #[starknet::contract]
            #[feature("forward-impl")]
            pub mod static_proxy {{
                #[storage] struct Storage {{}}
                impl CounterClassHash =
                    super::counter_contract__class_hash__::ForwardingClassHashImpl<ContractState>;
                #[abi(embed_v0)]
                impl Forwarded = super::ICounterForwardImpl<ContractState>;
            }}
            "#
        )
    };

    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(manifest)
        .dep_starknet()
        .lib_cairo(lib_cairo("self.counter.read() + amount"))
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();
    let proxy_before = t
        .child("target/dev/hello_static_proxy.contract_class.json")
        .read_to_string();
    let counter_before = t
        .child("target/dev/hello_counter_contract.contract_class.json")
        .read_to_string();

    t.child("src/lib.cairo")
        .write_str(&lib_cairo("self.counter.read() + amount + 1"))
        .unwrap();

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();
    let proxy_after = t
        .child("target/dev/hello_static_proxy.contract_class.json")
        .read_to_string();
    let counter_after = t
        .child("target/dev/hello_counter_contract.contract_class.json")
        .read_to_string();

    let counter_before: ContractClass = serde_json::from_str(&counter_before).unwrap();
    let counter_after: ContractClass = serde_json::from_str(&counter_after).unwrap();
    assert_ne!(
        class_hash(&counter_before),
        class_hash(&counter_after),
        "target contract's own class hash should change when its logic changes"
    );

    // Exactly one differing position: the embedded constant, nothing else.
    let proxy_before: ContractClass = serde_json::from_str(&proxy_before).unwrap();
    let proxy_after: ContractClass = serde_json::from_str(&proxy_after).unwrap();
    let differing_positions = proxy_before
        .sierra_program
        .iter()
        .zip(proxy_after.sierra_program.iter())
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(
        differing_positions, 1,
        "facade's Sierra program should differ in exactly the embedded class-hash constant"
    );
}

fn class_hash(class: &ContractClass) -> starknet_core::types::Felt {
    let sierra: starknet_core::types::contract::SierraClass =
        serde_json::from_value(serde_json::to_value(class).unwrap()).unwrap();
    sierra.class_hash().unwrap()
}

// CASM output must still work alongside forwarding.
#[test]
fn forwarding_with_casm_output() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            forwarding = true
            casm = true
        "#})
        .dep_starknet()
        .lib_cairo(indoc! {r#"
            #[starknet::interface]
            pub trait ICounter<T> {
                fn increase_counter(ref self: T, amount: u128);
                fn get_counter(self: @T) -> u128;
            }

            #[starknet::contract]
            pub mod counter_contract {
                use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};
                #[storage]
                struct Storage { counter: u128 }
                #[abi(embed_v0)]
                impl CounterImpl of super::ICounter<ContractState> {
                    fn increase_counter(ref self: ContractState, amount: u128) {
                        self.counter.write(self.counter.read() + amount);
                    }
                    fn get_counter(self: @ContractState) -> u128 { self.counter.read() }
                }
            }

            #[starknet::contract]
            #[feature("forward-impl")]
            pub mod static_proxy {
                #[storage] struct Storage {}
                impl CounterClassHash =
                    super::counter_contract__class_hash__::ForwardingClassHashImpl<ContractState>;
                #[abi(embed_v0)]
                impl Forwarded = super::ICounterForwardImpl<ContractState>;
            }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target/dev/hello_static_proxy.compiled_contract_class.json")
        .assert_is_json::<CasmContractClass>();
    t.child("target/dev/hello_counter_contract.compiled_contract_class.json")
        .assert_is_json::<CasmContractClass>();
}

// A contract using a component must still be a valid forwarding target.
#[test]
fn forwarding_coexists_with_components() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            forwarding = true
        "#})
        .dep_starknet()
        .lib_cairo(indoc! {r#"
            #[starknet::interface]
            pub trait ICounter<T> {
                fn increase_counter(ref self: T, amount: u128);
                fn get_counter(self: @T) -> u128;
            }

            #[starknet::component]
            pub mod counter_component {
                use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};
                #[storage]
                struct Storage { counter: u128 }
                #[embeddable_as(CounterImpl)]
                impl Counter<
                    TContractState, +HasComponent<TContractState>
                > of super::ICounter<ComponentState<TContractState>> {
                    fn increase_counter(ref self: ComponentState<TContractState>, amount: u128) {
                        self.counter.write(self.counter.read() + amount);
                    }
                    fn get_counter(self: @ComponentState<TContractState>) -> u128 {
                        self.counter.read()
                    }
                }
            }

            #[starknet::contract]
            pub mod using_component {
                component!(path: super::counter_component, storage: counter, event: CounterEvent);
                #[storage]
                struct Storage {
                    #[substorage(v0)]
                    counter: super::counter_component::Storage,
                }
                #[event]
                #[derive(Drop, starknet::Event)]
                enum Event {
                    #[flat]
                    CounterEvent: super::counter_component::Event,
                }
                #[abi(embed_v0)]
                impl CounterImpl = super::counter_component::CounterImpl<ContractState>;
            }

            #[starknet::contract]
            #[feature("forward-impl")]
            pub mod component_proxy {
                #[storage] struct Storage {}
                impl UsingComponentClassHash =
                    super::using_component__class_hash__::ForwardingClassHashImpl<ContractState>;
                #[abi(embed_v0)]
                impl Forwarded = super::ICounterForwardImpl<ContractState>;
            }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target/dev/hello_using_component.contract_class.json")
        .assert_is_json::<ContractClass>();
    let proxy = t
        .child("target/dev/hello_component_proxy.contract_class.json")
        .assert_is_json::<ContractClass>();
    assert_eq!(proxy.entry_points_by_type.external.len(), 2);
}

// `forwarding = true` with nothing using it must not error or hang.
#[test]
fn forwarding_enabled_with_zero_usage() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
            forwarding = true
        "#})
        .dep_starknet()
        .lib_cairo(indoc! {r#"
            #[starknet::contract]
            pub mod plain_contract {
                #[storage]
                struct Storage {}
            }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..] Compiling hello v0.1.0 ([..])
        [..]  Finished `dev` profile target(s) in [..]
        "#});

    t.child("target/dev/hello_plain_contract.contract_class.json")
        .assert_is_json::<ContractClass>();
}
