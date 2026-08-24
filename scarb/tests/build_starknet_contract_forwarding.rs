use assert_fs::prelude::*;
use cairo_lang_starknet_classes::contract_class::ContractClass;
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
                    super::counter_contract::__class_hash__::ForwardingClassHashImpl<ContractState>;

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

#[test]
fn compile_multi_forwarding_contract_with_forwarding_dependency() {
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
                    super::counter_contract::__class_hash__::ForwardingClassHashImpl<ContractState>;

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
                    super::stats_proxy::__class_hash__::ForwardingClassHashImpl<ContractState>;
                impl FlagClassHash =
                    super::flag_contract::__class_hash__::ForwardingClassHashImpl<ContractState>;

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
